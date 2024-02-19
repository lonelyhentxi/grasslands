use lazy_static::lazy_static;
use nodejs_resolver::{Options as ResolverOptions, ResolveResult, Resolver};
use path_slash::PathExt;
use regex::Regex;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};
use sugar_path::SugarPath;

use crate::bindings::GrassImporter;

lazy_static! {
  pub static ref WEBPACK_TILDE_PATTERN_PREFIX: Regex = Regex::new(r"^~([^/]+)").unwrap();
}

#[derive(Debug)]
pub struct ImitatorImporter {
  pwd: PathBuf,
  file: Option<PathBuf>,
  sass_file_resolver: Arc<Resolver>,
  #[allow(dead_code)]
  sass_context_resolver: Arc<Resolver>,
}

impl ImitatorImporter {
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

  #[allow(dead_code)]
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

impl RefUnwindSafe for ImitatorImporter {}
unsafe impl Send for ImitatorImporter {}
unsafe impl Sync for ImitatorImporter {}

impl Unpin for ImitatorImporter {}

impl UnwindSafe for ImitatorImporter {}

impl grass::Fs for ImitatorImporter {
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

impl grass::CustomImporter for ImitatorImporter {
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

impl GrassImporter for ImitatorImporter {}
