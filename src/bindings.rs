use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi_derive::napi;
use std::fmt::Debug;

pub trait GrassImporter: grass::Fs + grass::CustomImporter {}

#[allow(non_camel_case_types)]
#[napi(string_enum)]
#[derive(Debug, Default)]
pub enum SassOutputStyle {
  #[default]
  expanded,
  compressed,
}

#[allow(non_camel_case_types)]
#[napi(string_enum)]
#[derive(Debug, Default)]
pub enum SassSyntax {
  idented,
  css,
  #[default]
  scss,
}

#[napi(object)]
pub struct SassImporterResult {
  pub contents: String,
  pub source_map_url: Option<String>,
  pub syntax: SassSyntax,
}

#[napi(object)]
pub struct SassCanonicalizeContext {
  pub containing_url: Option<String>,
  pub from_import: bool,
}

struct JsFunctionDebugWrap<'a> {
  name: Option<&'a str>,
}

impl<'a> JsFunctionDebugWrap<'a> {
  fn from_opt<A>(name: &'a str, a: &'a Option<A>) -> JsFunctionDebugWrap<'a> {
    match a {
      Some(_) => JsFunctionDebugWrap { name: Some(name) },
      None => JsFunctionDebugWrap { name: None },
    }
  }
}

impl<'a> Debug for JsFunctionDebugWrap<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(name) = self.name {
      f.write_str(format!("function {name}() {{<js-code>}}").as_str())
    } else {
      f.write_str("None")
    }
  }
}

#[allow(non_camel_case_types)]
#[napi(object, object_to_js = false)]
pub struct SassImporter {
  #[napi(ts_return_type = "string")]
  pub canonicalize:
    Option<ThreadsafeFunction<(String, Option<SassCanonicalizeContext>), ErrorStrategy::Fatal>>,
  #[napi(ts_return_type = "{ contents: string, syntax: string, sourceMapUrl?: string } | null")]
  pub load: Option<ThreadsafeFunction<String, ErrorStrategy::Fatal>>,
  #[napi(ts_return_type = "string | null")]
  pub find_file_url:
    Option<ThreadsafeFunction<(String, Option<SassCanonicalizeContext>), ErrorStrategy::Fatal>>,
}

impl Debug for SassImporter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SassImporter")
      .field(
        "canonicalize",
        &JsFunctionDebugWrap::from_opt("canonicalize", &self.canonicalize),
      )
      .field("load", &JsFunctionDebugWrap::from_opt("load", &self.load))
      .field(
        "findFileUrl",
        &JsFunctionDebugWrap::from_opt("findFileUrl", &self.find_file_url),
      )
      .finish()
  }
}

#[napi(object, object_to_js = false)]
#[derive(Debug, Default)]
pub struct SassOptions {
  pub file: Option<String>,
  pub data: Option<String>,
  pub load_paths: Option<Vec<String>>,
  pub syntax: Option<SassSyntax>,
  pub url: Option<String>,
  pub importer: Option<Vec<SassImporter>>,
  pub charset: Option<bool>,
  pub source_map: Option<bool>,
  pub source_map_include_sources: Option<bool>,
  pub style: Option<SassOutputStyle>,
  // @unimplemented
  // pub functions
  pub alert_ascii: Option<bool>,
  pub alert_color: Option<bool>,
  // @unimplemented
  // pub logger
  pub quiet_deps: Option<bool>,
  pub verbose: Option<bool>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct SassResult {
  pub css: String,
  pub loaded_urls: Vec<String>,
  pub source_map: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct SassSourceLocation {
  pub column: i16,
  pub line: i16,
  pub offset: i64,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct SassSourceSpan {
  pub context: Option<String>,
  pub end: Option<SassSourceLocation>,
  pub start: Option<SassSourceLocation>,
  pub text: String,
  // @type URL
  pub url: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct SassError {
  pub message: String,
  pub name: String,
  pub sass_message: String,
  pub sass_stack: String,
  pub span: SassSourceSpan,
  pub stack: Option<String>,
  // @unimplemented
  // pub prepareStackTrace
  pub stack_trace_limit: i64,
  // @unimplemented
  // pub to_string;
  // @unimplemented
  // pub captureStackTrace
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct SassCompileResult {
  pub success: Option<SassResult>,
  pub failure: Option<SassError>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct LegacySassStats {
  pub duration: i64,
  pub start: i64,
  pub end: i64,
  pub entry: String,
  pub included_files: Vec<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct LegacySassImportResult {
  pub file: Option<String>,
  pub contents: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct LegacyImporterThis {
  pub from_import: bool,
}

pub type LegacySassImporter =
  ThreadsafeFunction<(LegacyImporterThis, String, Option<String>), ErrorStrategy::Fatal>;

#[napi(object, object_to_js = false)]
#[derive(Default)]
pub struct LegacySassOptions {
  pub include_paths: Option<Vec<String>>,
  pub ident_type: Option<String>,
  pub ident_width: Option<i32>,
  pub linefeed: Option<String>,
  pub omit_source_map_url: Option<bool>,
  pub out_file: Option<String>,
  pub output_style: Option<SassOutputStyle>,
  pub source_map: Option<bool>,
  pub source_map_contents: Option<bool>,
  pub source_map_embed: Option<bool>,
  pub importer: Option<
    Vec<ThreadsafeFunction<(LegacyImporterThis, String, Option<String>), ErrorStrategy::Fatal>>,
  >,
  // @unimplemented
  // pub functions
  pub charset: Option<bool>,
  pub quiet_deps: Option<bool>,
  pub verbose: Option<bool>,
  // @unimplemented
  // pub logger
  pub data: Option<String>,
  pub file: Option<String>,
  pub indented_syntax: Option<bool>,
}

impl Debug for LegacySassOptions {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LegacySassOptions")
      .field("include_paths", &self.include_paths)
      .field("ident_type", &self.ident_type)
      .field("ident_width", &self.ident_width)
      .field("linefeed", &self.linefeed)
      .field("omit_source_map_url", &self.omit_source_map_url)
      .field("out_file", &self.out_file)
      .field("output_style", &self.output_style)
      .field("source_map", &self.source_map)
      .field("source_map_contents", &self.source_map_contents)
      .field("source_map_embed", &self.source_map_embed)
      .field(
        "importer",
        &JsFunctionDebugWrap::from_opt("importer", &self.importer),
      )
      .field("charset", &self.charset)
      .field("quiet_deps", &self.quiet_deps)
      .field("verbose", &self.verbose)
      .field("data", &self.data)
      .field("file", &self.file)
      .field("indented_syntax", &self.indented_syntax)
      .finish()
  }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct LegacySassResult {
  pub css: String,
  pub stats: LegacySassStats,
  pub map: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct LegacySassError {
  pub column: Option<u32>,
  pub file: Option<String>,
  pub formatted: Option<String>,
  pub line: Option<u32>,
  pub message: Option<String>,
  pub stack: Option<String>,
  pub name: String,
  pub status: u32,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct LegacySassCompileResult {
  pub success: Option<LegacySassResult>,
  pub failure: Option<LegacySassError>,
}
