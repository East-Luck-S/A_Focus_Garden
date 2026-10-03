import type { FlowerKind, LeafShape, PlantGenome, Rgb } from "../../shared/plant";

export const PLANT_VIEW_BOX = "0 0 240 280";

export type GrowthStage = "SEED" | "STEM" | "LEAVES" | "BRANCHES" | "FLOWER";

export type ScenePath = {
  key: string;
  d: string;
  stroke: string;
  strokeWidth: number;
  opacity: number;
  progress: number;
};

export type SceneLeaf = {
  key: string;
  cx: number;
  cy: number;
  size: number;
  angle: number;
  shape: LeafShape;
  fill: string;
  accent: string;
  opacity: number;
  progress: number;
};

export type SceneDot = {
  key: string;
  cx: number;
  cy: number;
  radius: number;
  fill: string;
  opacity: number;
  progress: number;
};

export type ScenePetal = {
  key: string;
  cx: number;
  cy: number;
  rx: number;
  ry: number;
  angle: number;
  kind: FlowerKind;
  fill: string;
  opacity: number;
  progress: number;
};

export type SceneFlowerHead = {
  key: string;
  cx: number;
  cy: number;
  radius: number;
  fill: string;
  accent: string;
  opacity: number;
  progress: number;
};

export type SceneEllipse = {
  cx: number;
  cy: number;
  rx: number;
  ry: number;
  fill: string;
  opacity: number;
  progress: number;
};

export type PlantScene = {
  viewBox: string;
  growth: number;
  stage: GrowthStage;
  morphology: PlantGenome["morphology"];
  seed: SceneDot;
  stem: ScenePath;
  branches: readonly ScenePath[];
  secondaryStems: readonly ScenePath[];
  tendrils: readonly ScenePath[];
  leaves: readonly SceneLeaf[];
  rosetteLeaves: readonly SceneLeaf[];
  buds: readonly SceneDot[];
  petals: readonly ScenePetal[];
  flowerHeads: readonly SceneFlowerHead[];
  flowerCenter: SceneDot;
  dewDrops: readonly SceneDot[];
  pollen: readonly SceneDot[];
  glow: number;
  groundShadow: SceneEllipse;
};

const BASE_X = 120;
const BASE_Y = 246;

/** Converts display growth to a safe stable value; missing/invalid growth means seed stage. */
export function clampGrowth(growth: number): number {
  if (Number.isNaN(growth)) return 0;
  if (growth === Number.POSITIVE_INFINITY) return 1;
  if (growth === Number.NEGATIVE_INFINITY) return 0;
  return Math.min(1, Math.max(0, growth));
}

function reveal(growth: number, start: number, end: number): number {
  return Math.min(1, Math.max(0, (growth - start) / (end - start)));
}

function stageFor(growth: number): GrowthStage {
  if (growth < 0.1) return "SEED";
  if (growth < 0.35) return "STEM";
  if (growth < 0.6) return "LEAVES";
  if (growth < 0.85) return "BRANCHES";
  return "FLOWER";
}

function rgb({ red, green, blue }: Rgb): string {
  const channel = (value: number) => Math.min(255, Math.max(0, Math.round(value)));
  return `rgb(${channel(red)} ${channel(green)} ${channel(blue)})`;
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

/** Returns the deterministic SVG outline for one projected leaf. */
export function buildLeafPath(leaf: SceneLeaf): string {
  if (leaf.shape === "ROUND") {
    return `M 0 0 C ${leaf.size * 0.18} ${-leaf.size * 0.48}, ${leaf.size * 0.7} ${-leaf.size * 0.48}, ${leaf.size * 0.72} 0 C ${leaf.size * 0.7} ${leaf.size * 0.48}, ${leaf.size * 0.18} ${leaf.size * 0.48}, 0 0 Z`;
  }
  const width = leaf.size * (leaf.shape === "LANCEOLATE" ? 0.2 : 0.3);
  const length = leaf.size * (leaf.shape === "LANCEOLATE" ? 0.78 : 0.58);
  if (leaf.shape === "SERRATED") {
    return `M 0 0 L ${length * 0.2} ${-width * 0.55} L ${length * 0.36} ${-width} L ${length * 0.52} ${-width * 0.48} L ${length * 0.7} ${-width * 0.82} L ${length} 0 L ${length * 0.7} ${width * 0.82} L ${length * 0.52} ${width * 0.48} L ${length * 0.36} ${width} L ${length * 0.2} ${width * 0.55} Z`;
  }
  return `M 0 0 C ${length * 0.35} ${-width}, ${length * 0.84} ${-width}, ${length} 0 C ${length * 0.84} ${width}, ${length * 0.35} ${width}, 0 0 Z`;
}

function stemPoint(t: number, height: number, curvature: number, breeze: number): { x: number; y: number } {
  const drift = curvature * t * t + breeze * t * 0.18;
  return { x: round(BASE_X + drift), y: round(BASE_Y - height * t) };
}

function leafScale(morphology: PlantGenome["morphology"]): number {
  switch (morphology) {
    case "FERN": return 0.78;
    case "SUCCULENT": return 0.98;
    case "WILDFLOWER": return 0.72;
    case "VINE": return 0.68;
    case "SHRUB": return 0.88;
  }
}

function branchLength(morphology: PlantGenome["morphology"], index: number): number {
  const base = morphology === "FERN" ? 46 : morphology === "SHRUB" ? 34 : morphology === "VINE" ? 29 : 24;
  return base - index * 2;
}

/**
 * Builds a serializable, deterministic SVG projection. It never writes to the
 * genome; React may rebuild it whenever external focus growth changes.
 */
export function buildPlantScene(genome: PlantGenome, requestedGrowth: number): PlantScene {
  const growth = clampGrowth(requestedGrowth);
  const stemProgress = reveal(growth, 0.1, 0.35);
  const leafProgress = reveal(growth, 0.35, 0.6);
  const branchProgress = reveal(growth, 0.6, 0.85);
  const flowerProgress = reveal(growth, 0.85, 1);
  const fullHeight = 64 + genome.stem.height * 1.18;
  const height = genome.morphology === "SUCCULENT" ? Math.min(58, fullHeight * 0.38) : fullHeight;
  const curvature = genome.stem.curvature * (genome.morphology === "VINE" ? 1.45 : 0.92);
  const breeze = genome.effects.breeze_offset;
  const top = stemPoint(1, height, curvature, breeze);
  const stemColor = rgb(genome.palette.stem);
  const leafPrimary = rgb(genome.palette.leaf_primary);
  const leafSecondary = rgb(genome.palette.leaf_secondary);
  const flowerPrimary = rgb(genome.palette.flower_primary);
  const flowerSecondary = rgb(genome.palette.flower_secondary);
  const scenePrefix = `v${genome.generator_version}-${genome.morphology.toLowerCase()}`;

  const stem: ScenePath = {
    key: `${scenePrefix}-stem`,
    d: `M ${BASE_X} ${BASE_Y} C ${round(BASE_X + breeze * 0.05)} ${round(BASE_Y - height * 0.3)}, ${round(BASE_X + curvature * 0.66)} ${round(BASE_Y - height * 0.72)}, ${top.x} ${top.y}`,
    stroke: stemColor,
    strokeWidth: Math.max(2, genome.stem.thickness * 0.72),
    opacity: stemProgress,
    progress: stemProgress,
  };

  const leaves = (genome.morphology === "SUCCULENT" ? [] : genome.leaves).flatMap((leaf, index) => {
    const t = Math.min(0.92, Math.max(0.12, leaf.position / 100));
    const anchor = stemPoint(t, height, curvature, breeze);
    const side = leaf.side === "LEFT" ? -1 : 1;
    const size = leaf.size * leafScale(genome.morphology) * (genome.morphology === "FERN" ? 0.86 + (index % 3) * 0.08 : 1);
    const angle = leaf.angle_degrees + (genome.morphology === "FERN" ? side * 10 : 0);
    const reach = size * (genome.morphology === "FERN" ? 0.82 : 0.39);
    const projected = {
      key: `${scenePrefix}-leaf-${index}-${leaf.shape.toLowerCase()}`,
      cx: round(anchor.x + side * reach),
      cy: round(anchor.y - size * (genome.morphology === "FERN" ? 0.18 + (index % 2) * 0.08 : 0.07)),
      size: round(size),
      angle,
      shape: leaf.shape,
      fill: index % 2 === 0 ? leafPrimary : leafSecondary,
      accent: index % 2 === 0 ? leafSecondary : leafPrimary,
      opacity: leafProgress,
      progress: leafProgress,
    } satisfies SceneLeaf;
    if (genome.morphology !== "FERN") return [projected];
    return [0, 1, 2].map((segment) => {
      const leafletSide = segment === 0 ? -1 : segment === 1 ? 1 : side;
      const leafletT = Math.min(0.94, Math.max(0.1, t + (segment === 2 ? 0.045 : -0.012)));
      const leafletAnchor = stemPoint(leafletT, height, curvature, breeze);
      const leafletSize = size * (segment === 2 ? 0.58 : 0.76);
      const leafletReach = leafletSize * (segment === 2 ? 0.5 : 0.82);
      return {
        ...projected,
        key: `${projected.key}-frond-${segment}`,
        cx: round(leafletAnchor.x + leafletSide * leafletReach),
        cy: round(leafletAnchor.y - leafletSize * 0.08),
        size: round(leafletSize),
        angle: leafletSide * (Math.abs(leaf.angle_degrees) + (segment === 2 ? 4 : 12)),
      };
    });
  });

  const branchProjection = Array.from({ length: genome.stem.branch_count }, (_, index) => {
    const t = 0.35 + ((index + 1) / (genome.stem.branch_count + 1)) * 0.55;
    const start = stemPoint(t, height, curvature, breeze);
    const side = (index + (genome.morphology === "SHRUB" ? 1 : 0)) % 2 === 0 ? -1 : 1;
    const length = branchLength(genome.morphology, index) + genome.stem.thickness;
    const lift = 14 + (index % 3) * 5;
    const endX = round(start.x + side * length);
    const endY = round(start.y - lift);
    return {
      endX,
      endY,
      scene: {
        key: `${scenePrefix}-branch-${index}`,
        d: `M ${start.x} ${start.y} Q ${round(start.x + side * length * 0.54)} ${round(start.y - lift * 0.18)}, ${endX} ${endY}`,
        stroke: stemColor,
        strokeWidth: Math.max(1.4, genome.stem.thickness * 0.44),
        opacity: branchProgress,
        progress: branchProgress,
      } satisfies ScenePath,
    };
  });
  const branches = genome.morphology === "SUCCULENT" ? [] : branchProjection.map(({ scene }) => scene);

  const secondaryStems = genome.morphology === "SHRUB" || genome.morphology === "WILDFLOWER"
    ? branchProjection.slice(0, Math.max(1, Math.ceil(genome.stem.branch_count / 2))).map(({ endX, endY, scene }, index) => ({
      ...scene,
      key: `${scenePrefix}-secondary-${index}`,
      d: `M ${BASE_X} ${BASE_Y} Q ${round((BASE_X + endX) / 2)} ${round(BASE_Y - height * 0.28)}, ${endX} ${endY}`,
      strokeWidth: Math.max(1.1, scene.strokeWidth * 0.82),
    }))
    : [];
  const tendrils = genome.morphology === "VINE"
    ? branchProjection.slice(0, Math.max(1, genome.stem.branch_count - 1)).map(({ endX, endY }, index) => ({
      key: `${scenePrefix}-tendril-${index}`,
      d: `M ${endX} ${endY} q ${8 + index * 2} ${-10 - index * 2}, ${12 + index * 3} 0 q ${4 + index} ${8 + index}, ${-5 - index} ${10 + index}`,
      stroke: stemColor,
      strokeWidth: Math.max(1, genome.stem.thickness * 0.25),
      opacity: branchProgress,
      progress: branchProgress,
    }))
    : [];
  const rosetteLeaves = genome.morphology === "SUCCULENT"
    ? genome.leaves.map((leaf, index) => {
      const angle = (360 / genome.leaves.length) * index + genome.flower.tilt_degrees;
      const radians = (angle * Math.PI) / 180;
      const size = Math.max(12, leaf.size * 0.92);
      return {
        key: `${scenePrefix}-rosette-${index}`,
        cx: round(BASE_X + Math.cos(radians) * size * 0.28),
        cy: round(BASE_Y - 8 + Math.sin(radians) * size * 0.16),
        size: round(size), angle, shape: leaf.shape,
        fill: index % 2 === 0 ? leafPrimary : leafSecondary,
        accent: index % 2 === 0 ? leafSecondary : leafPrimary,
        opacity: leafProgress, progress: leafProgress,
      } satisfies SceneLeaf;
    })
    : [];

  const buds = (genome.morphology === "SUCCULENT" ? [] : branchProjection).map(({ endX: cx, endY: cy }, index) => {
    return {
      key: `${scenePrefix}-bud-${index}`,
      cx,
      cy,
      radius: round(3 + (index % 2) + genome.flower.size * 0.045),
      fill: flowerSecondary,
      opacity: branchProgress,
      progress: branchProgress,
    } satisfies SceneDot;
  });

  const petals = Array.from({ length: genome.flower.kind === "CLUSTER" ? 0 : genome.flower.petal_count }, (_, index) => {
    const angle = genome.flower.tilt_degrees + (360 / genome.flower.petal_count) * index;
    const radius = genome.flower.size * (genome.flower.kind === "CLUSTER" ? 0.2 : 0.32);
    const radians = (angle * Math.PI) / 180;
    return {
      key: `${scenePrefix}-petal-${index}-${genome.flower.kind.toLowerCase()}`,
      cx: round(top.x + Math.cos(radians) * radius),
      cy: round(top.y + Math.sin(radians) * radius),
      rx: round(genome.flower.size * (genome.flower.kind === "BELL" ? 0.18 : 0.2)),
      ry: round(genome.flower.size * (genome.flower.kind === "STAR" ? 0.12 : genome.flower.kind === "CLUSTER" ? 0.14 : 0.28)),
      angle: genome.flower.kind === "BELL" ? angle + 90 : angle,
      kind: genome.flower.kind,
      fill: index % 2 === 0 ? flowerPrimary : flowerSecondary,
      opacity: flowerProgress,
      progress: flowerProgress,
    } satisfies ScenePetal;
  });
  const flowerHeads = genome.flower.kind === "CLUSTER"
    ? Array.from({ length: Math.min(5, Math.max(3, Math.ceil(genome.flower.petal_count / 2))) }, (_, index) => {
      const angle = (360 / Math.min(5, Math.max(3, Math.ceil(genome.flower.petal_count / 2)))) * index + genome.flower.tilt_degrees;
      const radians = (angle * Math.PI) / 180;
      return { key: `${scenePrefix}-head-${index}`, cx: round(top.x + Math.cos(radians) * genome.flower.size * 0.34), cy: round(top.y + Math.sin(radians) * genome.flower.size * 0.28), radius: round(Math.max(3, genome.flower.size * 0.16)), fill: index % 2 === 0 ? flowerPrimary : flowerSecondary, accent: flowerSecondary, opacity: flowerProgress, progress: flowerProgress } satisfies SceneFlowerHead;
    })
    : [];

  const dewDrops = Array.from({ length: genome.effects.dew_drops }, (_, index) => {
    const leaf = [...leaves, ...rosetteLeaves][index % Math.max(1, leaves.length + rosetteLeaves.length)];
    return {
      key: `${scenePrefix}-dew-${index}`,
      cx: round((leaf?.cx ?? top.x) + (index % 2 === 0 ? 2 : -2)),
      cy: round((leaf?.cy ?? top.y) - 2 - (index % 3)),
      radius: 1.5,
      fill: "rgb(225 239 237)",
      opacity: leafProgress * 0.8,
      progress: leafProgress,
    } satisfies SceneDot;
  });

  const pollen = Array.from({ length: genome.effects.pollen }, (_, index) => {
    const angle = genome.flower.tilt_degrees + index * 47;
    const distance = genome.flower.size * (0.54 + (index % 3) * 0.11);
    const radians = (angle * Math.PI) / 180;
    return {
      key: `${scenePrefix}-pollen-${index}`,
      cx: round(top.x + Math.cos(radians) * distance),
      cy: round(top.y + Math.sin(radians) * distance),
      radius: 1,
      fill: flowerSecondary,
      opacity: flowerProgress * 0.7,
      progress: flowerProgress,
    } satisfies SceneDot;
  });

  return {
    viewBox: PLANT_VIEW_BOX,
    growth,
    stage: stageFor(growth),
    morphology: genome.morphology,
    seed: {
      key: `${scenePrefix}-seed`, cx: BASE_X, cy: BASE_Y + 5, radius: 7, fill: stemColor, opacity: 0.35 + reveal(growth, 0, 0.1) * 0.65, progress: reveal(growth, 0, 0.1),
    },
    stem,
    branches,
    secondaryStems,
    tendrils,
    leaves,
    rosetteLeaves,
    buds,
    petals,
    flowerHeads,
    flowerCenter: {
      key: `${scenePrefix}-flower-center`, cx: top.x, cy: top.y, radius: round(3 + genome.flower.size * 0.12), fill: flowerSecondary, opacity: flowerProgress, progress: flowerProgress,
    },
    dewDrops,
    pollen,
    glow: Math.min(1, Math.max(0, genome.effects.glow / 100)) * flowerProgress,
    groundShadow: { cx: BASE_X, cy: BASE_Y + 8, rx: round(23 + genome.stem.thickness * 1.3), ry: 5, fill: "rgb(115 94 66)", opacity: 0.11 + leafProgress * 0.13, progress: leafProgress },
  };
}
