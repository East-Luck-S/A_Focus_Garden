import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { PlantGenome } from "../../shared/plant";

import { PlantSvg } from "./PlantSvg";
import { buildLeafPath, buildPlantScene, clampGrowth } from "./plant-scene";

const genome: PlantGenome = {
  generator_version: 1,
  morphology: "SHRUB",
  stem: { height: 73, thickness: 5, curvature: -14, branch_count: 4 },
  leaves: [
    { side: "LEFT", position: 18, angle_degrees: -36, size: 22, shape: "OVAL" },
    { side: "RIGHT", position: 38, angle_degrees: 42, size: 31, shape: "SERRATED" },
    { side: "LEFT", position: 58, angle_degrees: -51, size: 28, shape: "ROUND" },
    { side: "RIGHT", position: 78, angle_degrees: 32, size: 20, shape: "LANCEOLATE" },
  ],
  flower: { kind: "STAR", petal_count: 6, size: 26, tilt_degrees: -9 },
  palette: {
    stem: { red: 74, green: 103, blue: 65 },
    leaf_primary: { red: 92, green: 141, blue: 78 },
    leaf_secondary: { red: 155, green: 189, blue: 89 },
    flower_primary: { red: 242, green: 145, blue: 165 },
    flower_secondary: { red: 255, green: 220, blue: 139 },
  },
  effects: { glow: 55, dew_drops: 3, pollen: 4, breeze_offset: -6 },
};

describe("buildPlantScene", () => {
  it("is stable for the same genome and growth without mutating a frozen input", () => {
    const frozen = Object.freeze({
      ...genome,
      stem: Object.freeze({ ...genome.stem }),
      leaves: Object.freeze(genome.leaves.map((leaf) => Object.freeze({ ...leaf }))),
      flower: Object.freeze({ ...genome.flower }),
      palette: Object.freeze({
        ...genome.palette,
        stem: Object.freeze({ ...genome.palette.stem }),
        leaf_primary: Object.freeze({ ...genome.palette.leaf_primary }),
        leaf_secondary: Object.freeze({ ...genome.palette.leaf_secondary }),
        flower_primary: Object.freeze({ ...genome.palette.flower_primary }),
        flower_secondary: Object.freeze({ ...genome.palette.flower_secondary }),
      }),
      effects: Object.freeze({ ...genome.effects }),
    }) as PlantGenome;

    expect(buildPlantScene(frozen, 0.72)).toEqual(buildPlantScene(frozen, 0.72));
    expect(frozen).toEqual(genome);
  });

  it.each([
    [0, "SEED"], [0.1, "STEM"], [0.35, "LEAVES"], [0.6, "BRANCHES"], [0.85, "FLOWER"], [1, "FLOWER"],
  ])("maps growth %s to the %s stage", (growth, stage) => {
    expect(buildPlantScene(genome, growth).stage).toBe(stage);
  });

  it("hides each organ until its stage and fully reveals its progress at the stage end", () => {
    const atSeed = buildPlantScene(genome, 0);
    expect(atSeed.seed.opacity).toBeGreaterThan(0);
    expect(atSeed.stem.opacity).toBe(0);
    expect(atSeed.leaves.every((leaf) => leaf.opacity === 0)).toBe(true);
    expect(atSeed.dewDrops.every((dew) => dew.opacity === 0)).toBe(true);
    expect(atSeed.branches.every((branch) => branch.opacity === 0)).toBe(true);
    expect(atSeed.buds.every((bud) => bud.opacity === 0)).toBe(true);
    expect(atSeed.petals.every((petal) => petal.opacity === 0)).toBe(true);
    expect(atSeed.flowerCenter.opacity).toBe(0);
    expect(atSeed.pollen.every((grain) => grain.opacity === 0)).toBe(true);
    expect(atSeed.glow).toBe(0);

    const atStemStart = buildPlantScene(genome, 0.1);
    expect(atStemStart.stem.progress).toBe(0);
    expect(atStemStart.stem.opacity).toBe(0);
    const atLeavesStart = buildPlantScene(genome, 0.35);
    expect(atLeavesStart.stem.progress).toBe(1);
    expect(atLeavesStart.stem.opacity).toBe(1);
    expect(atLeavesStart.leaves.every((leaf) => leaf.progress === 0 && leaf.opacity === 0)).toBe(true);
    expect(atLeavesStart.dewDrops.every((dew) => dew.progress === 0 && dew.opacity === 0)).toBe(true);
    const atBranchesStart = buildPlantScene(genome, 0.6);
    expect(atBranchesStart.leaves.every((leaf) => leaf.progress === 1 && leaf.opacity === 1)).toBe(true);
    expect(atBranchesStart.dewDrops.every((dew) => dew.progress === 1 && dew.opacity > 0)).toBe(true);
    expect(atBranchesStart.branches.every((branch) => branch.progress === 0 && branch.opacity === 0)).toBe(true);
    expect(atBranchesStart.buds.every((bud) => bud.progress === 0 && bud.opacity === 0)).toBe(true);
    const atFlowerStart = buildPlantScene(genome, 0.85);
    expect(atFlowerStart.branches.every((branch) => branch.progress === 1 && branch.opacity === 1)).toBe(true);
    expect(atFlowerStart.buds.every((bud) => bud.progress === 1 && bud.opacity === 1)).toBe(true);
    expect(atFlowerStart.petals.every((petal) => petal.progress === 0 && petal.opacity === 0)).toBe(true);
    expect(atFlowerStart.flowerCenter.progress).toBe(0);
    expect(atFlowerStart.flowerCenter.opacity).toBe(0);
    expect(atFlowerStart.pollen.every((grain) => grain.progress === 0 && grain.opacity === 0)).toBe(true);
    expect(atFlowerStart.glow).toBe(0);
    const atFlowerEnd = buildPlantScene(genome, 1);
    expect(atFlowerEnd.petals.every((petal) => petal.progress === 1 && petal.opacity === 1)).toBe(true);
    expect(atFlowerEnd.flowerCenter.progress).toBe(1);
    expect(atFlowerEnd.flowerCenter.opacity).toBe(1);
    expect(atFlowerEnd.pollen.every((grain) => grain.progress === 1 && grain.opacity > 0)).toBe(true);
    expect(atFlowerEnd.glow).toBeGreaterThan(0);
  });

  it("clamps finite and non-finite growth safely", () => {
    expect(clampGrowth(Number.NaN)).toBe(0);
    expect(clampGrowth(Number.NEGATIVE_INFINITY)).toBe(0);
    expect(clampGrowth(-3)).toBe(0);
    expect(clampGrowth(0.42)).toBe(0.42);
    expect(clampGrowth(7)).toBe(1);
    expect(clampGrowth(Number.POSITIVE_INFINITY)).toBe(1);
  });

  it("projects distinct genomes into visibly different structures", () => {
    const vine: PlantGenome = {
      ...genome,
      morphology: "VINE",
      stem: { height: 128, thickness: 9, curvature: 24, branch_count: 1 },
      flower: { kind: "BELL", petal_count: 4, size: 38, tilt_degrees: 18 },
      effects: { glow: 0, dew_drops: 0, pollen: 0, breeze_offset: 12 },
    };
    const shrub = buildPlantScene(genome, 1);
    const vineScene = buildPlantScene(vine, 1);

    expect(vineScene.stem.d).not.toBe(shrub.stem.d);
    expect(vineScene.branches).toHaveLength(1);
    expect(shrub.branches).toHaveLength(4);
    expect(vineScene.petals[0]?.kind).toBe("BELL");
    expect(vineScene.leaves[0]?.cx).not.toBe(shrub.leaves[0]?.cx);
  });

  it("gives every morphology a distinct deterministic structural signature", () => {
    const morphologies = ["FERN", "SUCCULENT", "WILDFLOWER", "VINE", "SHRUB"] as const;
    const scenes = Object.fromEntries(
      morphologies.map((morphology) => [morphology, buildPlantScene({ ...genome, morphology }, 1)]),
    ) as Record<(typeof morphologies)[number], ReturnType<typeof buildPlantScene>>;
    const signatures = morphologies.map((morphology) => {
      const scene = scenes[morphology];
      return JSON.stringify({
        stem: scene.stem.d,
        branches: scene.branches.length,
        secondaryStems: scene.secondaryStems.length,
        tendrils: scene.tendrils.length,
        leaves: scene.leaves.map(({ cx, cy, size }) => [cx, cy, size]),
        rosetteLeaves: scene.rosetteLeaves.map(({ cx, cy, size }) => [cx, cy, size]),
      });
    });

    expect(new Set(signatures).size).toBe(morphologies.length);
    expect(scenes.FERN.leaves).toHaveLength(genome.leaves.length * 3);
    expect(scenes.SUCCULENT.leaves).toHaveLength(0);
    expect(scenes.SUCCULENT.branches).toHaveLength(0);
    expect(scenes.SUCCULENT.buds).toHaveLength(0);
    expect(scenes.SUCCULENT.rosetteLeaves).toHaveLength(genome.leaves.length);
    expect(scenes.WILDFLOWER.secondaryStems.length).toBeGreaterThan(0);
    expect(scenes.VINE.tendrils.length).toBeGreaterThan(0);
    expect(scenes.SHRUB.secondaryStems.length).toBeGreaterThan(0);
  });

  it("builds four genuinely different leaf outlines", () => {
    const sourceLeaf = buildPlantScene(genome, 1).leaves[0]!;
    const shapes = ["OVAL", "LANCEOLATE", "SERRATED", "ROUND"] as const;
    const paths = shapes.map((shape) => buildLeafPath({ ...sourceLeaf, shape }));

    expect(new Set(paths).size).toBe(shapes.length);
    expect((paths[2]!.match(/ L /g) ?? []).length).toBeGreaterThan(6);
  });

  it("keeps morphology-specific organs inside their growth stages", () => {
    const succulentAtSeed = buildPlantScene({ ...genome, morphology: "SUCCULENT" }, 0);
    const succulentAtLeaves = buildPlantScene({ ...genome, morphology: "SUCCULENT" }, 0.6);
    expect(succulentAtSeed.rosetteLeaves.every(({ opacity, progress }) => opacity === 0 && progress === 0)).toBe(true);
    expect(succulentAtLeaves.rosetteLeaves.every(({ opacity, progress }) => opacity === 1 && progress === 1)).toBe(true);

    const vineAtBranchStart = buildPlantScene({ ...genome, morphology: "VINE" }, 0.6);
    const vineAtFlowerStart = buildPlantScene({ ...genome, morphology: "VINE" }, 0.85);
    expect(vineAtBranchStart.tendrils.every(({ opacity, progress }) => opacity === 0 && progress === 0)).toBe(true);
    expect(vineAtFlowerStart.tendrils.every(({ opacity, progress }) => opacity === 1 && progress === 1)).toBe(true);

    const shrubAtBranchStart = buildPlantScene({ ...genome, morphology: "SHRUB" }, 0.6);
    expect(shrubAtBranchStart.secondaryStems.every(({ opacity, progress }) => opacity === 0 && progress === 0)).toBe(true);

    const cluster = { ...genome, flower: { ...genome.flower, kind: "CLUSTER" as const } };
    expect(buildPlantScene(cluster, 0.85).flowerHeads.every(({ opacity, progress }) => opacity === 0 && progress === 0)).toBe(true);
    expect(buildPlantScene(cluster, 1).flowerHeads.every(({ opacity, progress }) => opacity === 1 && progress === 1)).toBe(true);
  });

  it("keeps all scene numbers finite and bounded effects safe", () => {
    const scene = buildPlantScene({
      ...genome,
      effects: { glow: 100, dew_drops: 7, pollen: 15, breeze_offset: 12 },
    }, 1);
    const numbers: number[] = [];
    const collectNumbers = (value: unknown) => {
      if (typeof value === "number") numbers.push(value);
      else if (Array.isArray(value)) value.forEach(collectNumbers);
      else if (value && typeof value === "object") Object.values(value).forEach(collectNumbers);
    };
    collectNumbers(scene);

    expect(numbers.every(Number.isFinite)).toBe(true);
    expect(scene.dewDrops).toHaveLength(7);
    expect(scene.pollen).toHaveLength(15);
    expect(scene.glow).toBe(1);
    for (const part of [scene.seed, scene.flowerCenter, ...scene.buds, ...scene.dewDrops, ...scene.pollen]) {
      expect(part.radius).toBeGreaterThan(0);
      expect(part.opacity).toBeGreaterThanOrEqual(0);
      expect(part.opacity).toBeLessThanOrEqual(1);
      expect(part.progress).toBeGreaterThanOrEqual(0);
      expect(part.progress).toBeLessThanOrEqual(1);
    }
  });
});

describe("PlantSvg", () => {
  it("is memoized and requires an explicit localized label", () => {
    const { getByRole } = render(<PlantSvg genome={genome} growth={1} label="测试植物" />);
    expect((PlantSvg as unknown as { $$typeof: symbol }).$$typeof).toBe(Symbol.for("react.memo"));
    expect(getByRole("img", { name: "测试植物" })).toBeInTheDocument();
  });

  it("renders a valid labelled SVG image without reusable SVG ids", () => {
    const { container } = render(<PlantSvg genome={genome} growth={1} label="A calm shrub" />);
    const image = screen.getByRole("img", { name: "A calm shrub" });

    expect(image.tagName).toBe("svg");
    expect(image).toHaveAttribute("viewBox", "0 0 240 280");
    expect(image).toHaveAttribute("data-growth-stage", "FLOWER");
    expect(image).toHaveAttribute("data-morphology", "SHRUB");
    expect(image).toHaveAttribute("data-flower-kind", "STAR");
    expect(container.querySelectorAll("path").length).toBeGreaterThan(4);
    expect(container.querySelectorAll("svg defs")).toHaveLength(0);
  });

  it.each([
    ["BLOOM", "ellipse", "petal"],
    ["BELL", "path", "petal"],
    ["STAR", "polygon", "petal"],
    ["CLUSTER", "g", "flower-head"],
  ] as const)("renders %s with its own SVG geometry", (kind, tagName, part) => {
    const { container } = render(
      <PlantSvg genome={{ ...genome, flower: { ...genome.flower, kind } }} growth={1} label="测试植物" />,
    );
    const geometry = container.querySelectorAll(`[data-plant-part="${part}"]`);

    expect(geometry.length).toBeGreaterThan(0);
    expect([...geometry].every((node) => node.tagName.toLowerCase() === tagName)).toBe(true);
    if (kind === "CLUSTER") {
      expect(container.querySelectorAll('[data-plant-part="petal"]')).toHaveLength(0);
    }
  });

  it.each([
    [0, []],
    [0.1, []],
    [0.35, ["stem"]],
    [0.6, ["stem", "leaf", "dew"]],
    [0.85, ["stem", "leaf", "dew", "branch", "bud"]],
    [1, ["glow", "stem", "branch", "leaf", "bud", "petal", "flower-center", "dew", "pollen"]],
  ])("uses matching SVG opacity at growth %s", (growth, visibleParts) => {
    const { container } = render(<PlantSvg genome={genome} growth={growth} label="测试植物" />);
    const visible = new Set(visibleParts);
    for (const part of ["glow", "stem", "branch", "leaf", "bud", "petal", "flower-center", "dew", "pollen"]) {
      const nodes = container.querySelectorAll(`[data-plant-part="${part}"]`);
      expect(nodes.length).toBeGreaterThan(0);
      for (const node of nodes) {
        const opacity = Number(node.getAttribute("opacity"));
        if (visible.has(part)) expect(opacity).toBeGreaterThan(0);
        else expect(opacity).toBe(0);
      }
    }
  });

  it.each([
    ["SUCCULENT", "rosette-leaf", 0, "0"],
    ["SUCCULENT", "rosette-leaf", 0.6, "1"],
    ["VINE", "tendril", 0.6, "0"],
    ["VINE", "tendril", 0.85, "1"],
    ["SHRUB", "secondary-stem", 0.6, "0"],
    ["SHRUB", "secondary-stem", 0.85, "1"],
  ] as const)("projects %s %s opacity at growth %s", (morphology, part, growth, opacity) => {
    const { container } = render(
      <PlantSvg genome={{ ...genome, morphology }} growth={growth} label="测试植物" />,
    );
    const nodes = container.querySelectorAll(`[data-plant-part="${part}"]`);
    expect(nodes.length).toBeGreaterThan(0);
    expect([...nodes].every((node) => node.getAttribute("opacity") === opacity)).toBe(true);
  });

  it("grows cluster heads only in the flower stage and keeps the ground shadow bounded", () => {
    const cluster = { ...genome, flower: { ...genome.flower, kind: "CLUSTER" as const } };
    const { container, rerender } = render(<PlantSvg genome={cluster} growth={0.85} label="测试植物" />);
    expect([...container.querySelectorAll('[data-plant-part="flower-head"]')]
      .every((node) => node.getAttribute("opacity") === "0")).toBe(true);
    rerender(<PlantSvg genome={cluster} growth={1} label="测试植物" />);
    expect([...container.querySelectorAll('[data-plant-part="flower-head"]')]
      .every((node) => node.getAttribute("opacity") === "1")).toBe(true);
    const shadowOpacity = Number(container.querySelector('[data-plant-part="ground-shadow"]')?.getAttribute("opacity"));
    expect(shadowOpacity).toBeGreaterThan(0);
    expect(shadowOpacity).toBeLessThanOrEqual(1);
  });
});
