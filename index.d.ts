/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export const enum SassOutputStyle {
  expanded = 'expanded',
  compressed = 'compressed',
}
export const enum SassSyntax {
  idented = 'idented',
  css = 'css',
  scss = 'scss',
}
export interface SassOptions {
  file?: string
  data?: string
  loadPaths?: Array<string>
  syntax?: SassSyntax
  url?: string
  charset?: boolean
  sourceMap?: boolean
  sourceMapIncludeSources?: boolean
  style?: SassOutputStyle
  alertAscii?: boolean
  alertColor?: boolean
  quietDeps?: boolean
  verbose?: boolean
  resolveAlias?: Record<string, Array<string>>
}
export interface SassResult {
  css: string
  loadedUrls: Array<string>
  sourceMap?: string
}
export interface SassSourceLocation {
  column: number
  line: number
  offset: number
}
export interface SassSourceSpan {
  context?: string
  end?: SassSourceLocation
  start?: SassSourceLocation
  text: string
  url?: string
}
export interface SassError {
  message: string
  name: string
  sassMessage: string
  sassStack: string
  span: SassSourceSpan
  stack?: string
  stackTraceLimit: number
}
export interface SassCompileResult {
  success?: SassResult
  failure?: SassError
}
export interface LegacySassStats {
  duration: number
  start: number
  end: number
  entry: string
  includedFiles: Array<string>
}
export interface LegacySassOptions {
  includePaths?: Array<string>
  identType?: string
  identWidth?: number
  linefeed?: string
  omitSourceMapUrl?: boolean
  outFile?: string
  outputStyle?: SassOutputStyle
  sourceMap?: boolean
  sourceMapContents?: boolean
  sourceMapEmbed?: boolean
  charset?: boolean
  quietDeps?: boolean
  verbose?: boolean
  data?: string
  file?: string
  indentedSyntax?: boolean
  resolveAlias?: Record<string, Array<string>>
}
export interface LegacySassResult {
  css: string
  stats: LegacySassStats
  map?: string
}
export interface LegacySassError {
  column?: number
  file?: string
  formatted?: string
  line?: number
  message?: string
  stack?: string
  name: string
  status: number
}
export interface LegacySassCompileResult {
  success?: LegacySassResult
  failure?: LegacySassError
}
export function compile(source: string, options?: SassOptions | undefined | null): SassCompileResult
export function compileLegacy(source: string, options?: LegacySassOptions | undefined | null): LegacySassCompileResult
