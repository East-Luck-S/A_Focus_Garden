export type PersistenceStatus = {
  initialized: boolean;
  schemaVersion: number;
  databaseFileName: string;
};
export type AppSettings = { defaultFocusDurationMs: number };

export type NativeError = {
  code: string;
  message: string;
};

export type SessionStatus = "ACTIVE" | "PAUSED" | "COMPLETED" | "ABANDONED" | "RECOVERY_REQUIRED";
export type RecoveryResolution = "CONTINUE" | "ABANDON";

export type FocusSessionSnapshot = {
  id: string;
  plannedDurationMs: number;
  startedAt: number;
  targetEndAt: number;
  pausedAt: number | null;
  totalPausedMs: number;
  completedAt: number | null;
  status: SessionStatus;
  pauseCount: number;
  createdAt: number;
  updatedAt: number;
  remainingMs: number;
  observedAt: number;
  /** 16-character lowercase hexadecimal u64; never a JavaScript number. */
  plantSeed: string;
  generatorVersion: number;
  plantGenome: import("../plant").PlantGenome;
};

export type GardenPlantSummary = {
  id: string;
  plantedAt: number;
  generatorVersion: number;
  plantGenome: import("../plant").PlantGenome;
};

export type GardenPlantDetail = GardenPlantSummary & {
  plantSeed: string;
  sourceSession: {
    id: string;
    plannedDurationMs: number;
    startedAt: number;
    targetEndAt: number;
    completedAt: number;
    totalPausedMs: number;
    pauseCount: number;
  };
};
