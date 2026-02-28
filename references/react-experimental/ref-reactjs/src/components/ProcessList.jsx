import { useState, useEffect } from 'react';

function ProcessList() {
  const [processes, setProcesses] = useState([]);
  const [sortBy, setSortBy] = useState('cpu');
  const [sortOrder, setSortOrder] = useState('desc');
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchProcesses = async () => {
      try {
        const response = await fetch('/api/processes?limit=10');
        const data = await response.json();
        setProcesses(data.processes || []);
        setIsLoading(false);
      } catch (err) {
        console.error('Failed to fetch processes:', err);
        setIsLoading(false);
      }
    };

    fetchProcesses();
    const interval = setInterval(fetchProcesses, 3000);
    return () => clearInterval(interval);
  }, []);

  const sortedProcesses = [...processes].sort((a, b) => {
    const aVal = a[sortBy];
    const bVal = b[sortBy];
    return sortOrder === 'desc' ? bVal - aVal : aVal - bVal;
  });

  const handleSort = (field) => {
    if (sortBy === field) {
      setSortOrder(sortOrder === 'desc' ? 'asc' : 'desc');
    } else {
      setSortBy(field);
      setSortOrder('desc');
    }
  };

  const SortIcon = ({ field }) => (
    <span className={`ml-1 ${sortBy === field ? 'text-avocado-500' : 'text-zinc-600'}`}>
      {sortBy === field ? (sortOrder === 'desc' ? '↓' : '↑') : '↕'}
    </span>
  );

  if (isLoading) {
    return (
      <div className="bg-zinc-900 rounded-xl border border-zinc-800 p-6">
        <div className="animate-pulse space-y-3">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-8 bg-zinc-800 rounded" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-zinc-900 rounded-xl border border-zinc-800 overflow-hidden">
      <div className="p-4 border-b border-zinc-800">
        <h3 className="text-sm font-medium text-zinc-400 uppercase tracking-wider flex items-center gap-2">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 6h16M4 10h16M4 14h16M4 18h16" />
          </svg>
          Top Processes
        </h3>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="bg-zinc-800/50 text-zinc-400 text-left">
              <th className="px-4 py-3 font-medium">Process</th>
              <th className="px-4 py-3 font-medium text-right">PID</th>
              <th 
                className="px-4 py-3 font-medium text-right cursor-pointer hover:text-white transition-colors"
                onClick={() => handleSort('cpu')}
              >
                CPU<SortIcon field="cpu" />
              </th>
              <th 
                className="px-4 py-3 font-medium text-right cursor-pointer hover:text-white transition-colors"
                onClick={() => handleSort('memory')}
              >
                Memory<SortIcon field="memory" />
              </th>
            </tr>
          </thead>
          <tbody>
            {sortedProcesses.length === 0 ? (
              <tr>
                <td colSpan={4} className="px-4 py-8 text-center text-zinc-500">
                  No processes found
                </td>
              </tr>
            ) : (
              sortedProcesses.map((proc, index) => (
                <tr 
                  key={proc.pid} 
                  className={`border-t border-zinc-800 hover:bg-zinc-800/50 transition-colors ${
                    index === 0 ? 'bg-zinc-800/30' : ''
                  }`}
                >
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-2">
                      <span className={`w-2 h-2 rounded-full ${
                        proc.cpu > 50 ? 'bg-red-500' : 
                        proc.cpu > 20 ? 'bg-amber-500' : 'bg-avocado-500'
                      }`} />
                      <span className="text-white font-medium truncate max-w-[150px]">
                        {proc.name}
                      </span>
                    </div>
                  </td>
                  <td className="px-4 py-3 text-right text-zinc-500 font-mono">
                    {proc.pid}
                  </td>
                  <td className="px-4 py-3 text-right">
                    <span className={`font-mono ${
                      proc.cpu > 50 ? 'text-red-400' : 
                      proc.cpu > 20 ? 'text-amber-400' : 'text-zinc-300'
                    }`}>
                      {proc.cpu.toFixed(1)}%
                    </span>
                  </td>
                  <td className="px-4 py-3 text-right">
                    <span className="text-zinc-300 font-mono">
                      {proc.memory.toFixed(1)}%
                    </span>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export default ProcessList;
