import { useState } from "react";

import { PlantSvg } from "../../components/plant";
import { getGardenPlant, nativeErrorMessage, type GardenPlantDetail, type GardenPlantSummary } from "../../shared/native";

type Props = { plants: GardenPlantSummary[]; loadError: unknown; onRetry: () => void };
const dateTime = (value: number) => new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));

export function GardenScreen({ plants, loadError, onRetry }: Props) {
  const [selected, setSelected] = useState<GardenPlantDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const open = (id: string) => { setError(null); void getGardenPlant(id).then(setSelected).catch((reason: unknown) => setError(nativeErrorMessage(reason))); };
  if (loadError) return <section className="session-view" aria-label="花园"><p className="notice" role="alert">花园暂时无法读取，不影响当前专注。{nativeErrorMessage(loadError)}</p><button className="quiet" onClick={onRetry}>重试读取花园</button></section>;
  return <section className="session-view" aria-label="花园"><p className="eyebrow">花园</p>{error && <p className="notice" role="alert">{error}</p>}{plants.length === 0 && <p>你的花园正在等待第一株完成专注后种下的植物。</p>}<div className="duration-options">{plants.map((plant) => <button className="duration" key={plant.id} onClick={() => open(plant.id)}><PlantSvg genome={plant.plantGenome} growth={1} label="完成专注后种下的植物" /><span>{dateTime(plant.plantedAt)}</span></button>)}</div>{selected && <article className="recovery"><PlantSvg genome={selected.plantGenome} growth={1} label="完成专注后种下的植物" /><p>种下时间：{dateTime(selected.plantedAt)}</p><p>计划时长：{Math.round(selected.sourceSession.plannedDurationMs / 60_000)} 分钟 · 暂停 {selected.sourceSession.pauseCount} 次</p></article>}</section>;
}
