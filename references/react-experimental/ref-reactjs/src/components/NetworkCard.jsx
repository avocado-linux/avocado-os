function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function NetworkCard({ interface: iface }) {
  return (
    <div className="bg-zinc-900 rounded-xl p-5 border border-zinc-800 hover:border-zinc-700 transition-colors">
      <div className="flex items-center gap-2 mb-4">
        <div className="w-2 h-2 bg-avocado-500 rounded-full"></div>
        <span className="font-medium text-white">{iface.name}</span>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div className="bg-zinc-800/50 rounded-lg p-3">
          <div className="flex items-center gap-1 text-zinc-500 text-xs mb-2">
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
            </svg>
            Received
          </div>
          <div className="text-avocado-400 font-mono font-medium">{formatBytes(iface.rxBytes)}</div>
          <div className="text-zinc-600 text-xs mt-1">{iface.rxPackets.toLocaleString()} packets</div>
        </div>
        <div className="bg-zinc-800/50 rounded-lg p-3">
          <div className="flex items-center gap-1 text-zinc-500 text-xs mb-2">
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 10l7-7m0 0l7 7m-7-7v18" />
            </svg>
            Transmitted
          </div>
          <div className="text-blue-400 font-mono font-medium">{formatBytes(iface.txBytes)}</div>
          <div className="text-zinc-600 text-xs mt-1">{iface.txPackets.toLocaleString()} packets</div>
        </div>
      </div>
    </div>
  );
}

export default NetworkCard;
