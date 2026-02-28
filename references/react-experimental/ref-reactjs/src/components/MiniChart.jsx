function MiniChart({ data, color = 'avocado', height = 60 }) {
  if (!data || data.length === 0) return null;

  const max = Math.max(...data, 100);
  const min = 0;
  const range = max - min || 1;
  
  const width = 200;
  const padding = 2;
  const chartWidth = width - padding * 2;
  const chartHeight = height - padding * 2;
  
  const points = data.map((value, index) => {
    const x = padding + (index / (data.length - 1 || 1)) * chartWidth;
    const y = padding + chartHeight - ((value - min) / range) * chartHeight;
    return `${x},${y}`;
  }).join(' ');

  const areaPoints = `${padding},${height - padding} ${points} ${width - padding},${height - padding}`;

  const colorMap = {
    avocado: { stroke: '#84cc16', fill: 'rgba(132, 204, 22, 0.1)' },
    blue: { stroke: '#3b82f6', fill: 'rgba(59, 130, 246, 0.1)' },
    purple: { stroke: '#a855f7', fill: 'rgba(168, 85, 247, 0.1)' },
    amber: { stroke: '#f59e0b', fill: 'rgba(245, 158, 11, 0.1)' },
    red: { stroke: '#ef4444', fill: 'rgba(239, 68, 68, 0.1)' },
  };

  const colors = colorMap[color] || colorMap.avocado;

  return (
    <svg 
      viewBox={`0 0 ${width} ${height}`} 
      className="w-full"
      style={{ height }}
      preserveAspectRatio="none"
    >
      {/* Grid lines */}
      <line x1={padding} y1={padding} x2={width - padding} y2={padding} stroke="#27272a" strokeWidth="1" />
      <line x1={padding} y1={height / 2} x2={width - padding} y2={height / 2} stroke="#27272a" strokeWidth="1" strokeDasharray="4" />
      <line x1={padding} y1={height - padding} x2={width - padding} y2={height - padding} stroke="#27272a" strokeWidth="1" />
      
      {/* Area fill */}
      <polygon points={areaPoints} fill={colors.fill} />
      
      {/* Line */}
      <polyline
        points={points}
        fill="none"
        stroke={colors.stroke}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      
      {/* Current value dot */}
      {data.length > 0 && (
        <circle
          cx={width - padding}
          cy={padding + chartHeight - ((data[data.length - 1] - min) / range) * chartHeight}
          r="3"
          fill={colors.stroke}
        />
      )}
    </svg>
  );
}

export default MiniChart;
