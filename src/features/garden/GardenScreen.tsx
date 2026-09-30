import { useEffect, useRef, useState } from "react";

import { PlantSvg } from "../../components/plant";
import {
  getGardenOverview,
  getGardenPlant,
  listGardenPlants,
  nativeErrorMessage,
  type GardenCursor,
  type GardenOverview,
  type GardenPlantDetail,
  type GardenPlantPage,
  type GardenPlantSummary,
} from "../../shared/native";

type Props = { refreshKey: number };

const PAGE_SIZE = 24;
type FailedGardenRequest = "FIRST_PAGE" | "MORE" | null;

function dateTime(value: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function minutes(value: number): string {
  return `${Math.round(value / 60_000)} 分钟`;
}

function appendUnique(
  current: GardenPlantSummary[],
  next: GardenPlantSummary[],
): GardenPlantSummary[] {
  const known = new Set(current.map((plant) => plant.id));
  return [...current, ...next.filter((plant) => !known.has(plant.id))];
}

export function GardenScreen({ refreshKey }: Props) {
  const [plants, setPlants] = useState<GardenPlantSummary[]>([]);
  const [overview, setOverview] = useState<GardenOverview | null>(null);
  const [nextCursor, setNextCursor] = useState<GardenCursor | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [failedRequest, setFailedRequest] = useState<FailedGardenRequest>(null);
  const [selected, setSelected] = useState<GardenPlantDetail | null>(null);
  const [detailError, setDetailError] = useState<unknown>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const lastTriggerRef = useRef<HTMLButtonElement>(null);

  const loadFirstPage = () => {
    setLoading(true);
    setError(null);
    setFailedRequest(null);
    void Promise.all([listGardenPlants(null, PAGE_SIZE), getGardenOverview()])
      .then(([page, nextOverview]: [GardenPlantPage, GardenOverview]) => {
        setPlants(page.items);
        setNextCursor(page.nextCursor);
        setOverview(nextOverview);
      })
      .catch((reason: unknown) => {
        setError(reason);
        setFailedRequest("FIRST_PAGE");
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    let current = true;
    void Promise.all([listGardenPlants(null, PAGE_SIZE), getGardenOverview()])
      .then(([page, nextOverview]: [GardenPlantPage, GardenOverview]) => {
        if (!current) return;
        setPlants(page.items);
        setNextCursor(page.nextCursor);
        setOverview(nextOverview);
      })
      .catch((reason: unknown) => {
        if (current) {
          setError(reason);
          setFailedRequest("FIRST_PAGE");
        }
      })
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
    };
  }, [refreshKey]);

  useEffect(() => {
    if (selected) {
      closeButtonRef.current?.focus();
    }
  }, [selected]);

  const loadMore = () => {
    if (!nextCursor || loadingMore) return;
    setLoadingMore(true);
    setError(null);
    setFailedRequest(null);
    void listGardenPlants(nextCursor, PAGE_SIZE)
      .then((page) => {
        setPlants((current) => appendUnique(current, page.items));
        setNextCursor(page.nextCursor);
      })
      .catch((reason: unknown) => {
        setError(reason);
        setFailedRequest("MORE");
      })
      .finally(() => setLoadingMore(false));
  };

  const openDetail = (event: React.MouseEvent<HTMLButtonElement>, id: string) => {
    lastTriggerRef.current = event.currentTarget;
    setDetailError(null);
    setLoadingDetail(true);
    void getGardenPlant(id)
      .then(setSelected)
      .catch((reason: unknown) => setDetailError(reason))
      .finally(() => setLoadingDetail(false));
  };

  const closeDetail = () => {
    setSelected(null);
    setDetailError(null);
    lastTriggerRef.current?.focus();
  };

  const handleDialogKeyDown = (event: React.KeyboardEvent<HTMLElement>) => {
    if (event.key === "Tab") {
      const focusable = Array.from(
        event.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"),
      );
      const currentIndex = focusable.indexOf(document.activeElement as HTMLButtonElement);
      if (currentIndex === -1 || focusable.length === 0) return;
      event.preventDefault();
      const offset = event.shiftKey ? -1 : 1;
      const nextIndex = (currentIndex + offset + focusable.length) % focusable.length;
      focusable[nextIndex]?.focus();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeDetail();
    }
  };

  return (
    <section className="session-view garden-view" aria-label="花园">
      <p className="eyebrow">花园</p>
      <h2>你的专注花园</h2>

      {loading ? <p aria-live="polite">正在整理花园…</p> : null}

      {error ? (
        <div className="garden-error" role="alert">
          <p>花园暂时无法读取，不会影响当前专注。{nativeErrorMessage(error)}</p>
          <button
            className="quiet"
            type="button"
            onClick={failedRequest === "MORE" ? loadMore : loadFirstPage}
            disabled={loading || loadingMore}
          >
            {failedRequest === "MORE" ? "重试加载更多" : "重试读取花园"}
          </button>
        </div>
      ) : null}

      {overview && !loading ? (
        <dl className="garden-overview" aria-label="花园概览">
          <div>
            <dt>植物总数</dt>
            <dd>{overview.plantCount}</dd>
          </div>
          <div>
            <dt>累计计划专注</dt>
            <dd>{minutes(overview.totalPlannedDurationMs)}</dd>
          </div>
        </dl>
      ) : null}

      {!loading && !error && plants.length === 0 ? (
        <p className="garden-empty">你的花园正在等待第一株完成专注后种下的植物。</p>
      ) : null}

      {plants.length > 0 ? (
        <div className="garden-grid" aria-label="已种下的植物">
          {plants.map((plant) => (
            <button
              className="garden-plant-card"
              key={plant.id}
              type="button"
              onClick={(event) => openDetail(event, plant.id)}
              disabled={loadingDetail}
            >
              <PlantSvg
                genome={plant.plantGenome}
                growth={1}
                className="garden-plant"
                label="完成专注后种下的植物"
              />
              <span className="garden-plant-date">{dateTime(plant.plantedAt)}</span>
              <span className="garden-plant-facts">计划 {minutes(plant.sourcePlannedDurationMs)} · 暂停 {plant.sourcePauseCount} 次</span>
            </button>
          ))}
        </div>
      ) : null}

      {nextCursor ? (
        <div className="actions">
          <button className="quiet" type="button" onClick={loadMore} disabled={loadingMore}>
            {loadingMore ? "正在加载…" : "加载更多"}
          </button>
        </div>
      ) : null}

      {loadingDetail ? <p className="busy-status" aria-live="polite">正在打开植物详情…</p> : null}
      {detailError ? <p className="notice" role="alert">{nativeErrorMessage(detailError)}</p> : null}

      {selected ? (
        <div className="dialog-backdrop">
          <section
            className="confirm-dialog garden-detail-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="garden-detail-title"
            onKeyDown={handleDialogKeyDown}
          >
            <button
              className="quiet detail-close"
              ref={closeButtonRef}
              type="button"
              onClick={closeDetail}
            >
              关闭详情
            </button>
            <h2 id="garden-detail-title">这株植物的专注记录</h2>
            <PlantSvg
              genome={selected.plantGenome}
              growth={1}
              className="completion-plant"
              label="完成专注后种下的植物"
            />
            <dl className="garden-detail-facts">
              <div>
                <dt>种下时间</dt>
                <dd>{dateTime(selected.plantedAt)}</dd>
              </div>
              <div>
                <dt>计划时长</dt>
                <dd>{minutes(selected.sourceSession.plannedDurationMs)}</dd>
              </div>
              <div>
                <dt>暂停次数</dt>
                <dd>{selected.sourceSession.pauseCount} 次</dd>
              </div>
            </dl>
          </section>
        </div>
      ) : null}
    </section>
  );
}
