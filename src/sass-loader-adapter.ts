import { compile, compileLegacy, LegacySassError, LegacySassResult } from '../index'
import type { SassResult, LegacySassOptions, SassOptions } from '../index'

// options
type LoggerOptions = {
  deprecation: boolean
  span: { url: string; start: { line: number; column: number } }
  stack: string
}

type ResolveAliasEntry = string | Array<string>
type ResolveAliasOptions = Array<[string, ResolveAliasEntry]> | Record<string, ResolveAliasEntry>

type Options = Omit<SassOptions, 'resolveAlias'> & {
  importers: { canonicalize: () => Promise<void>; load: () => void }[]
  logger: {
    debug: (message: string, options: LoggerOptions) => void
    warn: (message: string, options: LoggerOptions) => void
  }
  url: URL
  resolveAlias: ResolveAliasOptions
}

type LegacyOptions = Omit<LegacySassOptions, 'resolveAlias'> & {
  importer: ((originalUrl: string, prev: any, done: any) => void)[]
  logger: {
    debug: (message: string, options: LoggerOptions) => void
    warn: (message: string, options: LoggerOptions) => void
  }
  resolveAlias: ResolveAliasOptions
}

export const info = `dart-sass\t1.69.5`

export function normalizeResolveAlias(options: ResolveAliasOptions | null): Record<string, Array<string>> | undefined {
  let items: Array<[string, Array<string>]> = []
  if (Array.isArray(options)) {
    items = options.map(([k, v]) => [k, Array.isArray(v) ? v : [v]])
  } else if (typeof options === 'object' && options) {
    items = Object.entries(options).map(([k, v]) => [k, Array.isArray(v) ? v : [v]])
  }
  if (!items.length) {
    return undefined
  }
  return Object.fromEntries(items)
}

export async function compileStringAsync(source: string, options?: Options): Promise<SassResult> {
  const ret = compile(
    source,
    options
      ? {
        ...options,
        file: (options.file ?? '').toString(),
        resolveAlias: normalizeResolveAlias(options.resolveAlias),
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

export function render(
  options: LegacyOptions,
  callback: (error?: LegacySassError, result?: LegacySassResult) => void,
): void {
  const ret = compileLegacy(
    options.data ?? '',
    options
      ? {
        ...options,
        resolveAlias: normalizeResolveAlias(options.resolveAlias),
      }
      : undefined,
  )

  if (ret.success) {
    return callback(undefined, ret.success)
  }

  return callback(ret.failure, undefined)
}
