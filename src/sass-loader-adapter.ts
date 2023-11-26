import { compile, compileLegacy, LegacySassError, LegacySassResult } from '../index'
import type { SassResult, LegacySassOptions, SassOptions } from '../index'

// options
type LoggerOptions = {
  deprecation: boolean
  span: { url: string; start: { line: number; column: number } }
  stack: string
}

type Options = SassOptions & {
  importers: { canonicalize: () => Promise<void>; load: () => void }[]
  logger: {
    debug: (message: string, options: LoggerOptions) => void
    warn: (message: string, options: LoggerOptions) => void
  }
  url: URL
}

type LegacyOptions = LegacySassOptions & {
  importer: ((originalUrl: string, prev: any, done: any) => void)[]
  logger: {
    debug: (message: string, options: LoggerOptions) => void
    warn: (message: string, options: LoggerOptions) => void
  }
}

export const info = `dart-sass\t1.69.5`

export function compileStringSync(source: string, options?: Options): SassResult {
  const ret = compile(source, options ? { ...options, file: (options.file ?? '').toString() } : undefined)

  if (ret.success) {
    return ret.success
  }

  // eslint-disable-next-line @typescript-eslint/no-throw-literal
  throw ret.failure
}

export async function compileStringAsync(source: string, options?: Options): Promise<SassResult> {
  const ret = compile(source, options ? { ...options, file: (options.file ?? '').toString() } : undefined)

  if (ret.success) {
    // eslint-disable-next-line @typescript-eslint/return-await,@typescript-eslint/await-thenable
    return await ret.success
  }

  // eslint-disable-next-line @typescript-eslint/no-throw-literal
  throw ret.failure
}

export function render(
  options: LegacyOptions,
  callback: (error?: LegacySassError, result?: LegacySassResult) => void,
): void {
  const ret = compileLegacy(options.data ?? '', options)

  if (ret.success) {
    return callback(undefined, ret.success)
  }

  return callback(ret.failure, undefined)
}
