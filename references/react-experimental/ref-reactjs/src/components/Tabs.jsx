function Tabs({ tabs, activeTab, onChange }) {
  return (
    <div className="flex border-b border-zinc-800">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onChange(tab.id)}
          className={`px-4 py-3 text-sm font-medium transition-colors relative ${
            activeTab === tab.id
              ? 'text-avocado-500'
              : 'text-zinc-500 hover:text-white'
          }`}
        >
          <span className="flex items-center gap-2">
            {tab.icon}
            {tab.label}
          </span>
          {activeTab === tab.id && (
            <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-avocado-500" />
          )}
        </button>
      ))}
    </div>
  );
}

export default Tabs;
