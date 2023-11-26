#![deny(clippy::all)]

use lazy_static::lazy_static;
use napi_derive::napi;
use nodejs_resolver::{Options as ResolverOptions, ResolveResult, Resolver};
use path_slash::PathExt;
use regex::Regex;
use std::fmt::Debug;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::{fs, io};
use sugar_path::SugarPath;

lazy_static! {
  pub static ref WEBPACK_TILDE_PATTERN_PREFIX: Regex = Regex::new(r"^~([^/]+)").unwrap();
}

#[derive(Debug)]
pub struct GrasslandsImporter {
  pwd: PathBuf,
  file: Option<PathBuf>,
  sass_file_resolver: Arc<Resolver>,
  sass_context_resolver: Arc<Resolver>,
}

impl GrasslandsImporter {
  pub fn new(file: Option<String>) -> Self {
    let sass_file_resolver = Resolver::new(ResolverOptions {
      extensions: vec![
        ".sass".to_owned(),
        ".scss".to_owned(),
        ".css".to_owned(),
        ".import.sass".to_owned(),
        ".import.scss".to_owned(),
        ".import.css".to_owned(),
      ],
      prefer_relative: false,
      external_cache: None,
      symlinks: true,
      resolve_to_context: false,
      main_files: vec!["_index".to_owned(), "index".to_owned()],
      ..Default::default()
    });
    let sass_module_resolver = Resolver::new(ResolverOptions {
      extensions: vec![
        ".sass".to_owned(),
        ".scss".to_owned(),
        ".css".to_owned(),
        ".import.sass".to_owned(),
        ".import.scss".to_owned(),
        ".import.css".to_owned(),
      ],
      prefer_relative: false,
      external_cache: None,
      symlinks: true,
      resolve_to_context: true,
      main_files: vec!["_index".to_owned(), "index".to_owned()],
      ..Default::default()
    });
    Self {
      file: file.map(PathBuf::from),
      pwd: std::env::current_dir().unwrap(),
      sass_file_resolver: Arc::new(sass_file_resolver),
      sass_context_resolver: Arc::new(sass_module_resolver),
    }
  }

  pub fn normalize_import_path(&self, import_path: &str) -> PathBuf {
    let import_path_with_slash = Path::new(import_path).to_slash_lossy();
    PathBuf::from(import_path_with_slash.to_string())
  }

  pub fn sass_file_resolve(&self, current_path: &Path, import_path: &str) -> Option<PathBuf> {
    let current_path = current_path.absolutize();
    let import_path = self.normalize_import_path(import_path);

    if let Ok(ResolveResult::Resource(resource)) = self
      .sass_file_resolver
      .resolve(&current_path, &import_path.to_string_lossy())
    {
      return Some(resource.path);
    }

    if let (Some(dirname), Some(basename)) = (import_path.parent(), import_path.file_name()) {
      let partial_path = PathBuf::from(dirname).join(format!("_{}", basename.to_string_lossy()));
      let partial_path = self.normalize_import_path(&partial_path.to_string_lossy());
      if let Ok(ResolveResult::Resource(resource)) = self
        .sass_file_resolver
        .resolve(&current_path, &partial_path.to_string_lossy())
      {
        return Some(resource.path);
      }
    }
    None
  }

  pub fn sass_context_resolve(&self, current_path: &Path, import_path: &str) -> Option<PathBuf> {
    let import_path = &self.normalize_import_path(import_path);
    let current_path = current_path.absolutize();
    if let Ok(ResolveResult::Resource(resource)) = self
      .sass_context_resolver
      .resolve(&current_path, &import_path.to_string_lossy())
    {
      Some(resource.path)
    } else {
      None
    }
  }
}

impl RefUnwindSafe for GrasslandsImporter {}
unsafe impl Send for GrasslandsImporter {}
unsafe impl Sync for GrasslandsImporter {}

impl Unpin for GrasslandsImporter {}

impl UnwindSafe for GrasslandsImporter {}

impl grass::Fs for GrasslandsImporter {
  #[inline]
  fn is_dir(&self, path: &Path) -> bool {
    path.is_dir()
  }

  #[inline]
  fn is_file(&self, path: &Path) -> bool {
    path.is_file()
  }

  #[inline]
  /// Read the entire contents of a file into a bytes vector.
  fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
  }

  #[inline]
  /// Canonicalize a file path
  fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
    dunce::canonicalize(path.normalize())
  }
}

impl grass::CustomImporter for GrasslandsImporter {
  #[tracing::instrument]
  fn find_import(
    &self,
    current_path: &Path,
    import_path: &Path,
    load_paths: &[PathBuf],
  ) -> Option<PathBuf> {
    let current_context = &{
      let mut p = current_path;
      if p.to_string_lossy().eq("stdin") {
        p = self.file.as_ref().unwrap_or(&self.pwd);
      }
      if p.is_dir() {
        p.to_path_buf()
      } else if let Some(p) = p.parent() {
        p.to_path_buf()
      } else {
        self.pwd.to_path_buf()
      }
    };
    let import_path_str = import_path.to_string_lossy();
    if import_path.is_absolute() {
      if let Ok(ResolveResult::Resource(resource)) = self
        .sass_file_resolver
        .resolve(current_context, &import_path.to_string_lossy())
      {
        Some(resource.path.to_path_buf())
      } else {
        None
      }
    } else if WEBPACK_TILDE_PATTERN_PREFIX.is_match(&import_path_str) {
      let de_tilde_path = WEBPACK_TILDE_PATTERN_PREFIX.replace(&import_path_str, "$1");

      self.sass_file_resolve(current_context, &de_tilde_path)
    } else {
      let relative_path_resolved_path =
        self.sass_file_resolve(current_context, &import_path.to_string_lossy());

      if relative_path_resolved_path.is_some() {
        return relative_path_resolved_path;
      }

      for p in load_paths {
        let load_path_relative_path = p.join(import_path);

        let load_path_resolved_path =
          self.sass_file_resolve(current_context, &load_path_relative_path.to_string_lossy());

        if load_path_resolved_path.is_some() {
          return load_path_resolved_path;
        }
      }

      None
    }
  }
}

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
#[derive(Clone, Debug, Default)]
pub struct SassOptions {
  pub file: Option<String>,
  pub data: Option<String>,
  pub load_paths: Option<Vec<String>>,
  pub syntax: Option<SassSyntax>,
  pub url: Option<String>,
  // @unimplemented
  // pub importer
  pub charset: Option<bool>,
  pub source_map: Option<bool>,
  pub source_map_include_sources: Option<bool>,
  pub style: Option<SassOutputStyle>,
  // @unimplemented
  // pub functions
  // @unimplemented
  // pub importers
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
#[derive(Clone, Debug, Default)]
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
  // @unimplemented
  // pub importer
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

pub fn get_entries_of_node_modules(
  file: Option<String>,
  paths: Option<Vec<String>>,
) -> Vec<PathBuf> {
  let paths = if let Some(paths) = paths {
    paths
  } else {
    vec![]
  };

  let mut entries = vec![];

  if let Some(f) = &file {
    let p = PathBuf::from(f);
    if let Some(p) = p.parent() {
      entries.push(p.to_path_buf());
    }
  }

  Vec::extend(&mut entries, paths.iter().map(PathBuf::from));

  entries
}

pub fn sass_to_grass_options<'a>(
  opts: SassOptions,
  fs: &'a dyn grass::Fs,
  importer: &'a dyn grass::CustomImporter,
) -> grass::Options<'a> {
  grass::Options::default()
    .style(if let Some(s) = opts.style {
      match &s {
        SassOutputStyle::compressed => grass::OutputStyle::Compressed,
        SassOutputStyle::expanded => grass::OutputStyle::Expanded,
      }
    } else {
      grass::OutputStyle::Expanded
    })
    .load_paths(&get_entries_of_node_modules(opts.file, opts.load_paths))
    .input_syntax(if let Some(s) = opts.syntax {
      match s {
        SassSyntax::css => grass::InputSyntax::Css,
        SassSyntax::idented => grass::InputSyntax::Sass,
        SassSyntax::scss => grass::InputSyntax::Scss,
      }
    } else {
      grass::InputSyntax::Scss
    })
    .allows_charset(opts.charset.unwrap_or(true))
    .quiet(opts.quiet_deps.unwrap_or(false))
    .fs(fs)
    .custom_importer(Some(importer))
}

pub fn legacy_sass_to_grass_options<'a>(
  opts: LegacySassOptions,
  fs: &'a dyn grass::Fs,
  importer: &'a dyn grass::CustomImporter,
) -> grass::Options<'a> {
  grass::Options::default()
    .style(if let Some(s) = opts.output_style {
      match &s {
        SassOutputStyle::compressed => grass::OutputStyle::Compressed,
        SassOutputStyle::expanded => grass::OutputStyle::Expanded,
      }
    } else {
      grass::OutputStyle::Expanded
    })
    .load_paths(&get_entries_of_node_modules(opts.file, opts.include_paths))
    .input_syntax(if let Some(s) = opts.indented_syntax {
      match s {
        true => grass::InputSyntax::Sass,
        false => grass::InputSyntax::Scss,
      }
    } else {
      grass::InputSyntax::Scss
    })
    .allows_charset(opts.charset.unwrap_or(true))
    .quiet(opts.quiet_deps.unwrap_or(false))
    .fs(fs)
    .custom_importer(Some(importer))
}

#[tracing::instrument]
pub fn to_sass_error(options: &SassOptions, err: grass::Error) -> SassError {
  use grass::ErrorKind::*;
  match err.kind() {
    ParseError {
      message,
      loc,
      unicode: _,
    } => {
      let context = loc.file.source_slice(
        loc
          .file
          .line_span(loc.begin.line)
          .merge(loc.file.line_span(loc.end.line)),
      );
      let text = loc.file.source_slice({
        let begin_span = loc.file.line_span(loc.begin.line);
        let begin_span = begin_span.subspan(loc.begin.column as u64, begin_span.len());
        let end_span = loc.file.line_span(loc.end.line);
        let end_span = end_span.subspan(0, loc.end.column as u64);
        begin_span.merge(end_span)
      });
      let file = options.file.clone().unwrap_or_default();
      let stack = format!(
        "ParseError: {} at {}:{}:{}-{}:{} {}",
        &message, &file, loc.begin.line, loc.begin.column, loc.end.line, loc.end.column, text
      );
      let err = SassError {
        message: message.clone(),
        name: "ParseError".to_string(),
        sass_message: message,
        sass_stack: stack.clone(),
        span: SassSourceSpan {
          context: Some(context.to_string()),
          end: Some(SassSourceLocation {
            column: loc.end.column as i16,
            line: loc.end.line as i16,
            offset: 0,
          }),
          start: Some(SassSourceLocation {
            column: loc.begin.column as i16,
            line: loc.begin.line as i16,
            offset: 0,
          }),
          text: text.to_string(),
          url: Some(loc.file.name().to_string()),
        },
        stack: Some(stack),
        stack_trace_limit: 1,
      };
      eprintln!("{:?}", &err);
      err
    }
    grass::ErrorKind::IoError(err) => {
      panic!("{}", err)
    }
    grass::ErrorKind::FromUtf8Error(err) => {
      panic!("{}", err)
    }
    _ => unimplemented!(),
  }
}

#[tracing::instrument]
pub fn to_legacy_sass_error(options: &LegacySassOptions, err: grass::Error) -> LegacySassError {
  use grass::ErrorKind::*;
  match err.kind() {
    ParseError {
      message,
      loc,
      unicode: _,
    } => {
      let text = loc.file.source_slice({
        let begin_span = loc.file.line_span(loc.begin.line);
        let begin_span = begin_span.subspan(loc.begin.column as u64, begin_span.len());
        let end_span = loc.file.line_span(loc.end.line);
        let end_span = end_span.subspan(0, loc.end.column as u64);
        begin_span.merge(end_span)
      });
      let file = options.file.clone().unwrap_or_default();
      let stack = format!(
        "ParseError: {} at {}:{}:{}-{}:{} {}",
        &message, &file, loc.begin.line, loc.begin.column, loc.end.line, loc.end.column, text
      );
      let err = LegacySassError {
        column: Some(loc.begin.column as u32),
        file: Some(file),
        formatted: Some(message.clone()),
        line: Some(loc.begin.line as u32),
        message: Some(message),
        stack: Some(stack),
        name: "ParseError".to_string(),
        status: 1,
      };
      eprintln!("{:?}", &err);
      err
    }
    grass::ErrorKind::IoError(err) => {
      panic!("{}", err)
    }
    grass::ErrorKind::FromUtf8Error(err) => {
      panic!("{}", err)
    }
    _ => unimplemented!(),
  }
}

#[napi]
pub fn compile(source: String, options: Option<SassOptions>) -> SassCompileResult {
  let options = options.unwrap_or_default();
  let grass_fs = Arc::new(GrasslandsImporter::new(options.file.clone()));
  let grass_opts = sass_to_grass_options(options.clone(), grass_fs.as_ref(), grass_fs.as_ref());
  let ret = grass::from_string(source, &grass_opts);

  match ret {
    Ok(css) => SassCompileResult {
      success: Some(SassResult {
        css,
        loaded_urls: vec![],
        source_map: None,
      }),
      failure: None,
    },
    Err(err) => SassCompileResult {
      success: None,
      failure: Some(to_sass_error(&options, *err)),
    },
  }
}

#[napi]
pub fn compile_legacy(
  source: String,
  options: Option<LegacySassOptions>,
) -> LegacySassCompileResult {
  let options = options.unwrap_or_default();
  let start_time = SystemTime::now();
  let entry = if let Some(file) = &options.file {
    PathBuf::from(file)
      .absolutize()
      .to_string_lossy()
      .to_string()
  } else {
    "data".to_string()
  };
  let grass_fs = Arc::new(GrasslandsImporter::new(options.file.clone()));
  let grass_opts =
    legacy_sass_to_grass_options(options.clone(), grass_fs.as_ref(), grass_fs.as_ref());
  let ret = grass::from_string(source, &grass_opts);
  let end_time = SystemTime::now();
  let duration_time = end_time.duration_since(start_time);

  match ret {
    Ok(css) => LegacySassCompileResult {
      success: Some(LegacySassResult {
        css,
        stats: LegacySassStats {
          duration: duration_time
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_millis() as i64,
          start: start_time
            .elapsed()
            .map_or_else(|_| 0, |d| d.as_millis() as i64),
          end: 0,
          entry,
          // TODO: not supported now
          included_files: vec![],
        },
        map: None,
      }),
      failure: None,
    },
    Err(err) => LegacySassCompileResult {
      success: None,
      failure: Some(to_legacy_sass_error(&options, *err)),
    },
  }
}
