import { useId, useMemo } from "react";

import type { PlantGenome } from "../../shared/plant";

import { buildLeafPath, buildPlantScene, type ScenePetal } from "./plant-scene";

export type PlantSvgProps = {
  genome: PlantGenome;
  growth: number;
  label?: string;
  className?: string;
};

function petalPoints(petal: ScenePetal, scale: number): string {
  const points = Array.from({ length: 4 }, (_, index) => {
    const angle = ((petal.angle + index * 90) * Math.PI) / 180;
    const radius = (index % 2 === 0 ? petal.ry * 1.8 : petal.rx) * scale;
    return `${petal.cx + Math.cos(angle) * radius},${petal.cy + Math.sin(angle) * radius}`;
  });
  return points.join(" ");
}

/** A pure presentation of a Rust-authoritative PlantGenome and supplied growth. */
export function PlantSvg({ genome, growth, label, className }: PlantSvgProps) {
  const titleId = useId();
  const scene = useMemo(() => buildPlantScene(genome, growth), [genome, growth]);
  const accessibleLabel = label ?? `${scene.morphology.toLowerCase()} plant, ${scene.stage.toLowerCase()} growth stage`;

  return (
    <svg className={className} viewBox={scene.viewBox} role="img" aria-labelledby={titleId} data-growth-stage={scene.stage} data-morphology={scene.morphology} data-flower-kind={genome.flower.kind}>
      <title id={titleId}>{accessibleLabel}</title>
      <g data-plant-part="glow" opacity={scene.glow * 0.24} aria-hidden="true">
        <circle cx={scene.flowerCenter.cx} cy={scene.flowerCenter.cy} r={scene.flowerCenter.radius * (2 + scene.glow)} fill={scene.flowerCenter.fill} />
      </g>
      <ellipse data-plant-part="ground-shadow" cx={scene.groundShadow.cx} cy={scene.groundShadow.cy} rx={scene.groundShadow.rx} ry={scene.groundShadow.ry} fill={scene.groundShadow.fill} opacity={scene.groundShadow.opacity} />
      <circle data-plant-part="seed" cx={scene.seed.cx} cy={scene.seed.cy} r={scene.seed.radius * (0.35 + scene.seed.progress * 0.65)} fill={scene.seed.fill} opacity={scene.seed.opacity} />
      <path data-plant-part="stem" d={scene.stem.d} fill="none" stroke={scene.stem.stroke} strokeWidth={scene.stem.strokeWidth} strokeLinecap="round" opacity={scene.stem.opacity} pathLength={1} strokeDasharray={1} strokeDashoffset={1 - scene.stem.progress} />
      {scene.branches.map((branch) => <path data-plant-part="branch" key={branch.key} d={branch.d} fill="none" stroke={branch.stroke} strokeWidth={branch.strokeWidth} strokeLinecap="round" opacity={branch.opacity} pathLength={1} strokeDasharray={1} strokeDashoffset={1 - branch.progress} />)}
      {scene.secondaryStems.map((stem) => <path data-plant-part="secondary-stem" key={stem.key} d={stem.d} fill="none" stroke={stem.stroke} strokeWidth={stem.strokeWidth} strokeLinecap="round" opacity={stem.opacity} pathLength={1} strokeDasharray={1} strokeDashoffset={1 - stem.progress} />)}
      {scene.tendrils.map((tendril) => <path data-plant-part="tendril" key={tendril.key} d={tendril.d} fill="none" stroke={tendril.stroke} strokeWidth={tendril.strokeWidth} strokeLinecap="round" opacity={tendril.opacity} pathLength={1} strokeDasharray={1} strokeDashoffset={1 - tendril.progress} />)}
      {scene.leaves.map((leaf) => (
        <g data-plant-part="leaf" key={leaf.key} transform={`translate(${leaf.cx} ${leaf.cy}) rotate(${leaf.angle}) scale(${0.24 + leaf.progress * 0.76})`} opacity={leaf.opacity}>
          <path d={buildLeafPath(leaf)} fill={leaf.fill} stroke={leaf.accent} strokeWidth="0.8" strokeLinejoin="round" />
          <path d={`M 0 0 L ${leaf.size * 0.75} 0`} stroke={leaf.accent} strokeWidth="0.8" strokeLinecap="round" opacity="0.72" />
        </g>
      ))}
      {scene.rosetteLeaves.map((leaf) => <g data-plant-part="rosette-leaf" key={leaf.key} transform={`translate(${leaf.cx} ${leaf.cy}) rotate(${leaf.angle}) scale(${0.24 + leaf.progress * 0.76})`} opacity={leaf.opacity}><path d={buildLeafPath(leaf)} fill={leaf.fill} stroke={leaf.accent} strokeWidth="0.9" /><path d={`M 0 0 L ${leaf.size * 0.66} 0`} stroke={leaf.accent} strokeWidth="0.8" /></g>)}
      {scene.buds.map((bud) => <circle data-plant-part="bud" key={bud.key} cx={bud.cx} cy={bud.cy} r={bud.radius * (0.3 + bud.progress * 0.7)} fill={bud.fill} opacity={bud.opacity} />)}
      {scene.petals.map((petal) => {
        const scale = 0.25 + petal.progress * 0.75;
        return petal.kind === "STAR"
          ? <polygon data-plant-part="petal" key={petal.key} points={petalPoints(petal, scale)} fill={petal.fill} opacity={petal.opacity} transform={`rotate(${petal.angle} ${petal.cx} ${petal.cy})`} />
          : petal.kind === "BELL"
            ? <path data-plant-part="petal" key={petal.key} d={`M ${petal.cx - petal.rx * scale} ${petal.cy} Q ${petal.cx} ${petal.cy + petal.ry * scale * 2.4}, ${petal.cx + petal.rx * scale} ${petal.cy} Q ${petal.cx} ${petal.cy + petal.ry * scale * 0.7}, ${petal.cx - petal.rx * scale} ${petal.cy} Z`} fill={petal.fill} opacity={petal.opacity} transform={`rotate(${petal.angle} ${petal.cx} ${petal.cy})`} />
            : <ellipse data-plant-part="petal" key={petal.key} cx={petal.cx} cy={petal.cy} rx={petal.rx * scale} ry={petal.ry * scale} fill={petal.fill} opacity={petal.opacity} transform={`rotate(${petal.angle} ${petal.cx} ${petal.cy})`} />;
      })}
      {scene.flowerHeads.map((head) => {
        const scale = 0.3 + head.progress * 0.7;
        return <g data-plant-part="flower-head" key={head.key} opacity={head.opacity}><circle cx={head.cx} cy={head.cy} r={head.radius * scale} fill={head.fill} /><circle cx={head.cx} cy={head.cy} r={head.radius * 0.34 * scale} fill={head.accent} /></g>;
      })}
      <circle data-plant-part="flower-center" cx={scene.flowerCenter.cx} cy={scene.flowerCenter.cy} r={scene.flowerCenter.radius * (0.3 + scene.flowerCenter.progress * 0.7)} fill={scene.flowerCenter.fill} opacity={scene.flowerCenter.opacity} />
      {scene.dewDrops.map((dew) => <circle data-plant-part="dew" key={dew.key} cx={dew.cx} cy={dew.cy} r={dew.radius} fill={dew.fill} opacity={dew.opacity} />)}
      {scene.pollen.map((grain) => <circle data-plant-part="pollen" key={grain.key} cx={grain.cx} cy={grain.cy} r={grain.radius} fill={grain.fill} opacity={grain.opacity} />)}
    </svg>
  );
}
