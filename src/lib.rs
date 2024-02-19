#![feature(trait_upcasting)]

mod bindings;
mod imitator_importer;
mod proxy_importer;

use crate::imitator_importer::ImitatorImporter;
use bindings::{
  GrassImporter, LegacySassCompileResult, LegacySassError, LegacySassOptions, LegacySassResult,
  LegacySassStats, SassCompileResult, SassError, SassOptions, SassOutputStyle, SassResult,
  SassSourceLocation, SassSourceSpan, SassSyntax,
};
use napi::tokio::{runtime::Handle, task::spawn_blocking};
use napi_derive::napi;
use proxy_importer::ProxyImporter;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use sugar_path::SugarPath;

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
  opts: &'a SassOptions,
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
    .load_paths(&get_entries_of_node_modules(
      opts.file.clone(),
      opts.load_paths.clone(),
    ))
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
  opts: &'a LegacySassOptions,
  fs: &'a dyn GrassImporter,
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
    .load_paths(&get_entries_of_node_modules(
      opts.file.clone(),
      opts.include_paths.clone(),
    ))
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
pub async fn compile(source: String, options: Option<SassOptions>) -> SassCompileResult {
  let handle = Handle::current();
  spawn_blocking(move || {
    let mut options = options.unwrap_or_default();
    let importer = options.importer.take();
    let file = options.file.clone().map(PathBuf::from);
    let grass_fs = ProxyImporter::from_sass_options(handle, importer, file)
      .map(|i| Arc::new(i) as Arc<dyn GrassImporter>)
      .unwrap_or_else(|| Arc::new(ImitatorImporter::new(options.file.clone())));
    let grass_opts = sass_to_grass_options(&options, grass_fs.as_ref(), grass_fs.as_ref());
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
  })
  .await
  .unwrap()
}

#[napi]
pub async fn compile_legacy(
  source: String,
  options: Option<LegacySassOptions>,
) -> LegacySassCompileResult {
  let handle = Handle::current();
  spawn_blocking(move || {
    let mut options = options.unwrap_or_default();

    let start_time = SystemTime::now();
    let entry = if let Some(file) = &options.file {
      PathBuf::from(file)
        .absolutize()
        .to_string_lossy()
        .to_string()
    } else {
      "data".to_string()
    };
    let importer = options.importer.take();
    let file = options.file.clone().map(PathBuf::from);
    let grass_fs = ProxyImporter::from_legacy_sass_options(handle, importer, file)
      .map(|i| Arc::new(i) as Arc<dyn GrassImporter>)
      .unwrap_or_else(|| Arc::new(ImitatorImporter::new(options.file.clone())));
    let grass_opts = legacy_sass_to_grass_options(&options, grass_fs.as_ref(), grass_fs.as_ref());
    // println!("{} {:?}", &source, &grass_opts);
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
  })
  .await
  .unwrap()
}
