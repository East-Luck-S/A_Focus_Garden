export type PersistenceStatus = {
  initialized: boolean;
  schemaVersion: number;
  databaseFileName: string;
};

export type NativeError = {
  code: string;
  message: string;
};
