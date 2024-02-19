import {
  compile,
  compileLegacy,
  type LegacySassError,
  type LegacySassResult,
  SassResult,
  LegacySassOptions,
  SassOptions,
  SassImporter,
} from '../index'

// options
type LoggerOptions = {
  deprecation: boolean
  span: { url: string; start: { line: number; column: number } }
  stack: string
}

type Options = Omit<SassOptions, 'importer'> & {
  logger: {
    debug: (message: string, options: LoggerOptions) => void
    warn: (message: string, options: LoggerOptions) => void
  }
  url: URL
  importer?: SassImporter
  importers?: Array<SassImporter>
}

type LegacyOptions = Omit<LegacySassOptions, 'importer'> & {
  logger: {
    debug: (message: string, options: LoggerOptions) => void
    warn: (message: string, options: LoggerOptions) => void
  }
  importer?: (
    this: { fromImport: boolean },
    arg0: string,
    arg1: string | undefined | null,
    done: (res: any) => void,
  ) => any | Array<(arg0: string, arg1: string | undefined | null, done: (res: any) => void) => any>
}

export const info = `dart-sass\t1.69.5`

export async function compileStringAsync(source: string, options?: Options): Promise<SassResult> {
  const ret = await compile(
    source,
    options
      ? {
        ...options,
        file: (options.file ?? '').toString(),
        importer: options.importers?.length ? options.importers : options.importer ? [options.importer] : undefined,
      }
      : undefined,
  )

  if (ret.success) {
    // eslint-disable-next-line @typescript-eslint/return-await,@typescript-eslint/await-thenable
    return await ret.success
  }

  // eslint-disable-next-line @typescript-eslint/no-throw-literal
  throw ret.failure
}

export async function render(
  options: LegacyOptions,
  callback: (error?: LegacySassError, result?: LegacySassResult) => void,
) {
  const importers = Array.isArray(options.importer) ? options.importer : options.importer ? [options.importer] : []
  const importer = importers.length
    ? importers.map((fn) => {
      return async (importerThis: { fromImport: boolean }, arg0: string, arg1: string | undefined | null) => {
        return new Promise((resolve) => {
          fn.call(importerThis, arg0, arg1, resolve)
        })
      }
    })
    : undefined
  const ret = await compileLegacy(
    options.data ?? '',
    options
      ? {
        ...options,
        importer,
      }
      : null,
  )

  if (ret.success) {
    return callback(undefined, ret.success)
  }

  return callback(ret.failure, undefined)
}
