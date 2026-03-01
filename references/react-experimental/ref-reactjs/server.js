import express from 'express';
import { readFileSync, existsSync, readdirSync } from 'fs';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const app = express();
const PORT = process.env.PORT || 4000;

// Helper function to safely read proc files
function readProcFile(path) {
  try {
    if (existsSync(path)) {
      return readFileSync(path, 'utf8');
    }
  } catch (e) {
    console.error(`Error reading ${path}:`, e.message);
  }
  return null;
}

// Parse CPU stats from /proc/stat
function getCpuStats() {
  const stat = readProcFile('/proc/stat');
  if (!stat) return null;

  const lines = stat.split('\n');
  const cpuLine = lines.find(line => line.startsWith('cpu '));
  if (!cpuLine) return null;

  const parts = cpuLine.split(/\s+/).slice(1).map(Number);
  const [user, nice, system, idle, iowait, irq, softirq, steal] = parts;
  
  const total = user + nice + system + idle + iowait + irq + softirq + steal;
  const used = total - idle - iowait;
  
  // Get per-core stats
  const cores = [];
  for (const line of lines) {
    const match = line.match(/^cpu(\d+)\s+(.+)/);
    if (match) {
      const coreParts = match[2].split(/\s+/).map(Number);
      const coreTotal = coreParts.reduce((a, b) => a + b, 0);
      const coreIdle = coreParts[3] + coreParts[4];
      cores.push({
        core: parseInt(match[1]),
        percentage: coreTotal > 0 ? Math.round(((coreTotal - coreIdle) / coreTotal) * 100) : 0
      });
    }
  }
  
  return {
    total,
    used,
    idle: idle + iowait,
    user,
    system,
    percentage: total > 0 ? Math.round((used / total) * 100) : 0,
    cores
  };
}

// Parse memory stats from /proc/meminfo
function getMemoryStats() {
  const meminfo = readProcFile('/proc/meminfo');
  if (!meminfo) return null;

  const lines = meminfo.split('\n');
  const values = {};
  
  for (const line of lines) {
    const match = line.match(/^(\w+):\s+(\d+)/);
    if (match) {
      values[match[1]] = parseInt(match[2], 10);
    }
  }

  const total = values.MemTotal || 0;
  const free = values.MemFree || 0;
  const buffers = values.Buffers || 0;
  const cached = values.Cached || 0;
  const available = values.MemAvailable || (free + buffers + cached);
  const used = total - available;
  const swapTotal = values.SwapTotal || 0;
  const swapFree = values.SwapFree || 0;

  return {
    total: Math.round(total / 1024),
    used: Math.round(used / 1024),
    free: Math.round(free / 1024),
    available: Math.round(available / 1024),
    cached: Math.round(cached / 1024),
    buffers: Math.round(buffers / 1024),
    percentage: total > 0 ? Math.round((used / total) * 100) : 0,
    swap: {
      total: Math.round(swapTotal / 1024),
      free: Math.round(swapFree / 1024),
      used: Math.round((swapTotal - swapFree) / 1024),
      percentage: swapTotal > 0 ? Math.round(((swapTotal - swapFree) / swapTotal) * 100) : 0
    }
  };
}

// Parse load average from /proc/loadavg
function getLoadAverage() {
  const loadavg = readProcFile('/proc/loadavg');
  if (!loadavg) return null;

  const parts = loadavg.split(/\s+/);
  return {
    load1: parseFloat(parts[0]) || 0,
    load5: parseFloat(parts[1]) || 0,
    load15: parseFloat(parts[2]) || 0,
    runningProcesses: parts[3] || '0/0'
  };
}

// Parse uptime from /proc/uptime
function getUptime() {
  const uptime = readProcFile('/proc/uptime');
  if (!uptime) return null;

  const parts = uptime.split(/\s+/);
  const seconds = parseFloat(parts[0]) || 0;
  
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  return {
    seconds: Math.round(seconds),
    formatted: `${days}d ${hours}h ${minutes}m ${secs}s`,
    days,
    hours,
    minutes
  };
}

// Get disk usage by reading /proc/mounts and statvfs equivalent
function getDiskStats() {
  try {
    // Read /proc/mounts to find /var mount
    const mounts = readProcFile('/proc/mounts');
    if (!mounts) {
      console.error('Cannot read /proc/mounts');
      return null;
    }

    // Find /var or root mount
    const lines = mounts.split('\n');
    let targetMount = '/';
    for (const line of lines) {
      const parts = line.split(/\s+/);
      if (parts[1] === '/var') {
        targetMount = '/var';
        break;
      }
    }

    // Use stat to get filesystem info - read from /proc/self/mountinfo
    const mountinfo = readProcFile('/proc/self/mountinfo');
    if (mountinfo) {
      for (const line of mountinfo.split('\n')) {
        if (line.includes(targetMount) && !line.includes('overlay')) {
          // Found our mount, now try df
          break;
        }
      }
    }

    // Try running df with shell
    const result = execSync(`df -m ${targetMount} 2>&1 || df -m / 2>&1`, { 
      encoding: 'utf8',
      shell: '/bin/sh'
    });
    
    const dfLines = result.split('\n').filter(l => l.trim() && !l.startsWith('Filesystem'));
    if (dfLines.length > 0) {
      const parts = dfLines[0].trim().split(/\s+/);
      if (parts.length >= 5) {
        return {
          total: parseInt(parts[1], 10) || 0,
          used: parseInt(parts[2], 10) || 0,
          available: parseInt(parts[3], 10) || 0,
          percentage: parseInt((parts[4] || '0').replace('%', ''), 10) || 0,
          mountpoint: parts[5] || targetMount
        };
      }
    }
    
    return null;
  } catch (e) {
    console.error('getDiskStats error:', e.message, e.stack);
    return null;
  }
}

// Get network stats
function getNetworkStats() {
  const netDev = readProcFile('/proc/net/dev');
  if (!netDev) return null;

  const lines = netDev.split('\n').slice(2);
  const interfaces = [];

  for (const line of lines) {
    const match = line.match(/^\s*(\w+):\s*(.+)/);
    if (match && match[1] !== 'lo') {
      const parts = match[2].split(/\s+/).map(Number);
      interfaces.push({
        name: match[1],
        rxBytes: parts[0] || 0,
        rxPackets: parts[1] || 0,
        txBytes: parts[8] || 0,
        txPackets: parts[9] || 0
      });
    }
  }

  return interfaces;
}

// Get temperature (works on various Linux systems)
function getTemperature() {
  const tempPaths = [
    '/sys/class/thermal/thermal_zone0/temp',
    '/sys/class/hwmon/hwmon0/temp1_input',
    '/sys/devices/virtual/thermal/thermal_zone0/temp'
  ];

  for (const path of tempPaths) {
    const temp = readProcFile(path);
    if (temp) {
      const celsius = parseInt(temp, 10) / 1000;
      return {
        celsius: Math.round(celsius * 10) / 10,
        fahrenheit: Math.round((celsius * 9/5 + 32) * 10) / 10
      };
    }
  }
  return null;
}

// Get hostname
function getHostname() {
  const hostname = readProcFile('/etc/hostname');
  return hostname ? hostname.trim() : 'unknown';
}

// Get OS release info
function getOsRelease() {
  const osRelease = readProcFile('/etc/os-release');
  if (!osRelease) return { name: 'Linux', version: 'Unknown' };

  const values = {};
  for (const line of osRelease.split('\n')) {
    const match = line.match(/^(\w+)=["']?([^"'\n]+)["']?/);
    if (match) {
      values[match[1]] = match[2];
    }
  }

  return {
    name: values.NAME || values.ID || 'Linux',
    version: values.VERSION || values.VERSION_ID || 'Unknown',
    prettyName: values.PRETTY_NAME || values.NAME || 'Linux'
  };
}

// Get top processes by reading /proc directly (works on all Linux)
function getProcesses(limit = 10) {
  const processes = [];
  
  try {
    const procEntries = readdirSync('/proc');
    const pidDirs = procEntries.filter(name => /^\d+$/.test(name));
    
    for (const pid of pidDirs) {
      try {
        const commPath = `/proc/${pid}/comm`;
        const statusPath = `/proc/${pid}/status`;
        
        let name = 'unknown';
        let rss = 0;
        let state = 'S';
        
        // Read comm file
        try {
          if (existsSync(commPath)) {
            name = readFileSync(commPath, 'utf8').trim();
          }
        } catch (e) {
          // Process might have exited
          continue;
        }
        
        // Read status file for memory info
        try {
          if (existsSync(statusPath)) {
            const status = readFileSync(statusPath, 'utf8');
            const rssMatch = status.match(/VmRSS:\s*(\d+)\s*kB/i);
            if (rssMatch) {
              rss = Math.round(parseInt(rssMatch[1], 10) / 1024);
            }
            const stateMatch = status.match(/State:\s*(\w)/);
            if (stateMatch) {
              state = stateMatch[1];
            }
          }
        } catch (e) {
          // Ignore status read errors
        }
        
        if (name && name !== 'unknown') {
          processes.push({
            pid: parseInt(pid, 10),
            name,
            cpu: 0,
            memory: 0,
            rss,
            status: state
          });
        }
      } catch (e) {
        // Skip this process
      }
    }
  } catch (e) {
    console.error('getProcesses error reading /proc:', e.message);
  }
  
  // Sort by RSS descending and return top N
  return processes
    .filter(p => p.pid > 0)
    .sort((a, b) => b.rss - a.rss)
    .slice(0, limit);
}

// API endpoint for system stats
app.get('/api/stats', (req, res) => {
  res.json({
    timestamp: Date.now(),
    hostname: getHostname(),
    os: getOsRelease(),
    cpu: getCpuStats(),
    memory: getMemoryStats(),
    load: getLoadAverage(),
    uptime: getUptime(),
    disk: getDiskStats(),
    network: getNetworkStats(),
    temperature: getTemperature()
  });
});

// API endpoint for processes
app.get('/api/processes', (req, res) => {
  const limit = parseInt(req.query.limit) || 15;
  res.json({
    timestamp: Date.now(),
    processes: getProcesses(limit)
  });
});

// Serve static files from the dist directory
app.use(express.static(join(__dirname, 'dist')));

// Fallback to index.html for SPA routing
app.get('*', (req, res) => {
  res.sendFile(join(__dirname, 'dist', 'index.html'));
});

app.listen(PORT, '0.0.0.0', () => {
  console.log(`React.js reference app running on http://0.0.0.0:${PORT}`);
});
