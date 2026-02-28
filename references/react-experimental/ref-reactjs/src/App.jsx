import { useState, useEffect, useCallback } from 'react';
import StatCard from './components/StatCard';
import ProgressBar from './components/ProgressBar';
import NetworkCard from './components/NetworkCard';
import MiniChart from './components/MiniChart';
import ProcessList from './components/ProcessList';
import Tabs from './components/Tabs';
import useLocalStorage from './hooks/useLocalStorage';

function AvocadoLogo({ className = "w-10 h-10" }) {
  return (
    <svg className={className} viewBox="0 0 300 300" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M285.539 109.945C292.662 122.287 291.147 137.796 281.768 148.524L188.993 254.651C182.671 261.884 173.533 266.033 163.928 266.033L82.138 266.033C70.242 266.033 59.2495 259.685 53.3015 249.379L14.461 182.086C8.51299 171.781 8.51299 159.084 14.461 148.779L55.3558 77.9271C60.1585 69.6061 68.3196 63.7651 77.7432 61.9042L216.013 34.5994C229.99 31.8392 244.175 38.2802 251.298 50.6223L285.539 109.945Z" fill="currentColor" fillOpacity="0.3"/>
      <path fillRule="evenodd" clipRule="evenodd" d="M198.915 86.2748C202.48 86.3816 206.295 87.4311 209.229 90.4108C213.737 94.9382 213.834 101.324 212.803 106.572C211.727 112.087 209.069 118.331 205.423 124.806C203.494 128.266 201.227 131.895 198.666 135.631C201.489 146.223 200.675 157.456 196.355 167.53C192.034 177.604 184.456 185.935 174.838 191.186C163.926 197.092 151.194 198.679 139.166 195.634C135.465 198.211 131.64 200.606 127.706 202.812C121.188 206.432 114.911 209.055 109.381 210.105C104.099 211.11 97.7067 210.968 93.1989 206.458C90.2292 203.479 89.1534 199.618 89.0378 195.989C88.9311 192.396 89.7224 188.545 90.9939 184.693C93.4034 177.489 97.9468 169.101 103.993 160.322C101.892 152.083 101.973 143.439 104.23 135.242C106.486 127.044 110.839 119.577 116.86 113.575C122.881 107.574 130.361 103.246 138.563 101.018C146.766 98.7902 155.407 98.7395 163.635 100.871C172.393 95.0716 180.75 90.5709 187.908 88.1961C191.642 86.9508 195.394 86.177 198.915 86.2748ZM178.323 107.417C179.141 107.951 179.941 108.52 180.724 109.107L180.75 109.125L184.76 112.185C178.003 123.614 167.2 137.347 153.73 150.992C140.482 164.396 126.985 175.078 115.658 181.811L112.359 177.72C111.679 176.821 111.029 175.898 110.412 174.954C107.229 180.255 104.962 184.987 103.655 188.918C102.659 191.916 102.321 194.122 102.375 195.58C102.401 196.47 102.552 196.861 102.624 197.003C102.828 197.11 103.886 197.564 106.891 197.003C110.466 196.318 115.329 194.424 121.233 191.142C124.469 189.345 127.919 187.193 131.52 184.711C141.194 177.836 151.543 169.012 161.625 158.801C171.717 148.59 180.501 138.192 187.374 128.497C189.846 124.922 191.998 121.497 193.794 118.286C197.101 112.389 199.022 107.551 199.724 104.002C200.302 101.04 199.866 99.9992 199.76 99.8213C199.361 99.6751 198.939 99.6057 198.515 99.6167C197.137 99.5722 195.03 99.8835 192.113 100.862C188.237 102.143 183.578 104.358 178.332 107.417H178.323Z" fill="currentColor"/>
    </svg>
  );
}

const MAX_HISTORY = 60;

function App() {
  const [stats, setStats] = useState(null);
  const [error, setError] = useState(null);
  const [lastUpdate, setLastUpdate] = useState(null);
  const [activeTab, setActiveTab] = useState('overview');
  const [cpuHistory, setCpuHistory] = useState([]);
  const [memoryHistory, setMemoryHistory] = useState([]);
  const [settings, setSettings] = useLocalStorage('avocado-settings', {
    refreshInterval: 2000,
    showCharts: true,
  });

  const fetchStats = useCallback(async () => {
    try {
      const response = await fetch('/api/stats');
      if (!response.ok) throw new Error('Failed to fetch stats');
      const data = await response.json();
      setStats(data);
      setLastUpdate(new Date());
      setError(null);

      if (data.cpu) {
        setCpuHistory(prev => [...prev.slice(-(MAX_HISTORY - 1)), data.cpu.percentage]);
      }
      if (data.memory) {
        setMemoryHistory(prev => [...prev.slice(-(MAX_HISTORY - 1)), data.memory.percentage]);
      }
    } catch (err) {
      setError(err.message);
    }
  }, []);

  useEffect(() => {
    fetchStats();
    const interval = setInterval(fetchStats, settings.refreshInterval);
    return () => clearInterval(interval);
  }, [fetchStats, settings.refreshInterval]);

  const tabs = [
    {
      id: 'overview',
      label: 'Overview',
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
        </svg>
      )
    },
    {
      id: 'processes',
      label: 'Processes',
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 6h16M4 10h16M4 14h16M4 18h16" />
        </svg>
      )
    },
    {
      id: 'settings',
      label: 'Settings',
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      )
    }
  ];

  if (error) {
    return (
      <div className="min-h-screen bg-zinc-950 text-white flex items-center justify-center">
        <div className="text-center">
          <AvocadoLogo className="w-16 h-16 text-avocado-500 mx-auto mb-4" />
          <h1 className="text-2xl font-bold text-red-400 mb-4">Connection Error</h1>
          <p className="text-zinc-400 mb-4">{error}</p>
          <button
            onClick={fetchStats}
            className="px-4 py-2 bg-avocado-600 hover:bg-avocado-500 text-white rounded-lg transition-colors"
          >
            Retry Connection
          </button>
        </div>
      </div>
    );
  }

  if (!stats) {
    return (
      <div className="min-h-screen bg-zinc-950 text-white flex items-center justify-center">
        <div className="text-center">
          <AvocadoLogo className="w-16 h-16 text-avocado-500 mx-auto mb-4 animate-pulse" />
          <p className="text-zinc-400">Loading system stats...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-zinc-950 text-white">
      {/* Header */}
      <header className="border-b border-zinc-800 bg-zinc-900/50 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <AvocadoLogo className="w-10 h-10 text-avocado-500" />
              <div>
                <h1 className="text-xl font-semibold text-white">
                  Avocado <span className="text-avocado-500">System Monitor</span>
                </h1>
                <p className="text-sm text-zinc-500">
                  {stats.hostname} <span className="text-zinc-700">•</span> {stats.os?.prettyName || 'Avocado Linux'}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-avocado-500 rounded-full animate-pulse"></div>
                <span className="text-sm text-zinc-500">Live</span>
              </div>
              {lastUpdate && (
                <span className="text-xs text-zinc-600 hidden sm:block">
                  Updated {lastUpdate.toLocaleTimeString()}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="max-w-7xl mx-auto px-6">
          <Tabs tabs={tabs} activeTab={activeTab} onChange={setActiveTab} />
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-6 py-8">
        {activeTab === 'overview' && (
          <>
            {/* Primary Stats Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
              {/* CPU Usage */}
              <StatCard
                title="CPU Usage"
                icon={
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
                  </svg>
                }
              >
                {stats.cpu && (
                  <>
                    <div className="flex items-baseline gap-2 mb-3">
                      <span className="text-4xl font-bold text-white">{stats.cpu.percentage}</span>
                      <span className="text-2xl text-zinc-500">%</span>
                    </div>
                    <ProgressBar value={stats.cpu.percentage} color="avocado" />
                    {settings.showCharts && cpuHistory.length > 1 && (
                      <div className="mt-4">
                        <MiniChart data={cpuHistory} color="avocado" />
                      </div>
                    )}
                    {stats.cpu.cores && stats.cpu.cores.length > 1 && (
                      <div className="mt-4 grid grid-cols-4 gap-2">
                        {stats.cpu.cores.slice(0, 8).map((core) => (
                          <div key={core.core} className="text-center">
                            <div className="text-xs text-zinc-500 mb-1">C{core.core}</div>
                            <div className="h-1 bg-zinc-800 rounded-full overflow-hidden">
                              <div
                                className="h-full bg-avocado-500 transition-all duration-300"
                                style={{ width: `${core.percentage}%` }}
                              />
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </>
                )}
              </StatCard>

              {/* Memory Usage */}
              <StatCard
                title="Memory"
                icon={
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                }
              >
                {stats.memory && (
                  <>
                    <div className="flex items-baseline gap-2 mb-3">
                      <span className="text-4xl font-bold text-white">{stats.memory.percentage}</span>
                      <span className="text-2xl text-zinc-500">%</span>
                    </div>
                    <ProgressBar value={stats.memory.percentage} color="blue" />
                    {settings.showCharts && memoryHistory.length > 1 && (
                      <div className="mt-4">
                        <MiniChart data={memoryHistory} color="blue" />
                      </div>
                    )}
                    <div className="mt-4 grid grid-cols-3 gap-2 text-xs">
                      <div className="bg-zinc-800/50 rounded-lg p-2 text-center">
                        <div className="text-zinc-500">Used</div>
                        <div className="text-white font-medium">{stats.memory.used} MB</div>
                      </div>
                      <div className="bg-zinc-800/50 rounded-lg p-2 text-center">
                        <div className="text-zinc-500">Cached</div>
                        <div className="text-zinc-400">{stats.memory.cached} MB</div>
                      </div>
                      <div className="bg-zinc-800/50 rounded-lg p-2 text-center">
                        <div className="text-zinc-500">Total</div>
                        <div className="text-zinc-400">{stats.memory.total} MB</div>
                      </div>
                    </div>
                  </>
                )}
              </StatCard>

              {/* Disk Usage */}
              <StatCard
                title="Disk"
                icon={
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
                  </svg>
                }
              >
                {stats.disk && (
                  <>
                    <div className="flex items-baseline gap-2 mb-3">
                      <span className="text-4xl font-bold text-white">{stats.disk.percentage}</span>
                      <span className="text-2xl text-zinc-500">%</span>
                    </div>
                    <ProgressBar value={stats.disk.percentage} color="purple" />
                    <div className="mt-4 space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span className="text-zinc-500">Used</span>
                        <span className="text-white font-medium">{stats.disk.used} MB</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-zinc-500">Total</span>
                        <span className="text-zinc-400">{stats.disk.total} MB</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-zinc-500">Free</span>
                        <span className="text-zinc-400">{stats.disk.available} MB</span>
                      </div>
                    </div>
                  </>
                )}
              </StatCard>
            </div>

            {/* Secondary Stats Grid */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
              {/* Load Average */}
              <StatCard
                title="Load Average"
                icon={
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                }
              >
                {stats.load && (
                  <>
                    <div className="grid grid-cols-3 gap-3 text-center">
                      <div className="bg-zinc-800/50 rounded-lg p-3">
                        <div className="text-2xl font-bold text-avocado-400">{stats.load.load1.toFixed(2)}</div>
                        <div className="text-xs text-zinc-500 mt-1">1 min</div>
                      </div>
                      <div className="bg-zinc-800/50 rounded-lg p-3">
                        <div className="text-2xl font-bold text-avocado-400">{stats.load.load5.toFixed(2)}</div>
                        <div className="text-xs text-zinc-500 mt-1">5 min</div>
                      </div>
                      <div className="bg-zinc-800/50 rounded-lg p-3">
                        <div className="text-2xl font-bold text-avocado-400">{stats.load.load15.toFixed(2)}</div>
                        <div className="text-xs text-zinc-500 mt-1">15 min</div>
                      </div>
                    </div>
                    <div className="mt-3 text-sm text-zinc-500 text-center">
                      Processes: {stats.load.runningProcesses}
                    </div>
                  </>
                )}
              </StatCard>

              {/* Uptime */}
              <StatCard
                title="Uptime"
                icon={
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                }
              >
                {stats.uptime && (
                  <div className="text-center">
                    <div className="text-3xl font-bold text-white mb-2">
                      {stats.uptime.formatted}
                    </div>
                    <div className="text-sm text-zinc-500">
                      System has been running for
                    </div>
                    <div className="text-sm text-zinc-400">
                      {stats.uptime.days > 0 && `${stats.uptime.days} days, `}
                      {stats.uptime.hours} hours, {stats.uptime.minutes} minutes
                    </div>
                  </div>
                )}
              </StatCard>

              {/* Temperature */}
              <StatCard
                title="Temperature"
                icon={
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                  </svg>
                }
              >
                {stats.temperature ? (
                  <div className="text-center">
                    <div className={`text-4xl font-bold mb-2 ${
                      stats.temperature.celsius > 70 ? 'text-red-400' :
                      stats.temperature.celsius > 50 ? 'text-amber-400' : 'text-avocado-400'
                    }`}>
                      {stats.temperature.celsius}<span className="text-2xl">°C</span>
                    </div>
                    <div className="text-sm text-zinc-500">
                      {stats.temperature.fahrenheit}°F
                    </div>
                  </div>
                ) : (
                  <div className="text-center text-zinc-600">
                    <div className="text-2xl mb-2">—</div>
                    <div className="text-sm">Sensor not available</div>
                  </div>
                )}
              </StatCard>
            </div>

            {/* Network Stats */}
            {stats.network && stats.network.length > 0 && (
              <div>
                <h2 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                  <svg className="w-5 h-5 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
                  </svg>
                  Network Interfaces
                </h2>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {stats.network.map((iface) => (
                    <NetworkCard key={iface.name} interface={iface} />
                  ))}
                </div>
              </div>
            )}
          </>
        )}

        {activeTab === 'processes' && (
          <div className="space-y-6">
            <ProcessList />
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="max-w-2xl">
            <div className="bg-zinc-900 rounded-xl border border-zinc-800 divide-y divide-zinc-800">
              <div className="p-6">
                <h3 className="text-lg font-medium text-white mb-4">Display Settings</h3>
                <div className="space-y-4">
                  <label className="flex items-center justify-between">
                    <span className="text-zinc-300">Show history charts</span>
                    <button
                      onClick={() => setSettings({ ...settings, showCharts: !settings.showCharts })}
                      className={`relative w-12 h-6 rounded-full transition-colors ${
                        settings.showCharts ? 'bg-avocado-600' : 'bg-zinc-700'
                      }`}
                    >
                      <span
                        className={`absolute top-1 left-1 w-4 h-4 bg-white rounded-full transition-transform ${
                          settings.showCharts ? 'translate-x-6' : ''
                        }`}
                      />
                    </button>
                  </label>
                  <div>
                    <label className="block text-zinc-300 mb-2">Refresh interval</label>
                    <select
                      value={settings.refreshInterval}
                      onChange={(e) => setSettings({ ...settings, refreshInterval: Number(e.target.value) })}
                      className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-avocado-500"
                    >
                      <option value={1000}>1 second</option>
                      <option value={2000}>2 seconds</option>
                      <option value={5000}>5 seconds</option>
                      <option value={10000}>10 seconds</option>
                    </select>
                  </div>
                </div>
              </div>
              <div className="p-6">
                <h3 className="text-lg font-medium text-white mb-4">About</h3>
                <div className="space-y-2 text-sm text-zinc-400">
                  <p>Avocado System Monitor v0.1.0</p>
                  <p>A React.js reference application for Avocado Linux</p>
                  <p className="text-zinc-500">
                    Built with React, Vite, Tailwind CSS, and Express.js
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="border-t border-zinc-800 mt-12">
        <div className="max-w-7xl mx-auto px-6 py-6">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <AvocadoLogo className="w-6 h-6 text-avocado-500" />
              <span className="text-sm text-zinc-500">
                Avocado Linux • The Developer Friendly Embedded Linux
              </span>
            </div>
            <div className="flex items-center gap-4 text-sm text-zinc-600">
              <a
                href="https://avocadolinux.org"
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-avocado-500 transition-colors"
              >
                avocadolinux.org
              </a>
              <span className="text-zinc-700">•</span>
              <a
                href="https://github.com/avocado-linux"
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-avocado-500 transition-colors"
              >
                GitHub
              </a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;
