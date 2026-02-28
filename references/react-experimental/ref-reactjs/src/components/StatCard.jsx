function StatCard({ title, icon, children }) {
  return (
    <div className="bg-zinc-900 rounded-xl p-6 border border-zinc-800 hover:border-zinc-700 transition-colors">
      <div className="flex items-center gap-2 mb-4">
        <div className="text-zinc-500">
          {icon}
        </div>
        <h3 className="text-sm font-medium text-zinc-400 uppercase tracking-wider">
          {title}
        </h3>
      </div>
      <div>
        {children}
      </div>
    </div>
  );
}

export default StatCard;
