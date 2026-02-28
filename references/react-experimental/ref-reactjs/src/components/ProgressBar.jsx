const colorClasses = {
  avocado: 'bg-avocado-500',
  blue: 'bg-blue-500',
  purple: 'bg-purple-500',
  red: 'bg-red-500',
  amber: 'bg-amber-500',
  green: 'bg-green-500',
  cyan: 'bg-cyan-500',
};

const bgColorClasses = {
  avocado: 'bg-avocado-500/20',
  blue: 'bg-blue-500/20',
  purple: 'bg-purple-500/20',
  red: 'bg-red-500/20',
  amber: 'bg-amber-500/20',
  green: 'bg-green-500/20',
  cyan: 'bg-cyan-500/20',
};

function ProgressBar({ value, color = 'avocado' }) {
  const clampedValue = Math.min(100, Math.max(0, value));
  const fgClass = colorClasses[color] || colorClasses.avocado;
  const bgClass = bgColorClasses[color] || bgColorClasses.avocado;
  
  return (
    <div className={`w-full rounded-full h-2 overflow-hidden ${bgClass}`}>
      <div
        className={`h-full rounded-full transition-all duration-500 ease-out ${fgClass}`}
        style={{ width: `${clampedValue}%` }}
      />
    </div>
  );
}

export default ProgressBar;
