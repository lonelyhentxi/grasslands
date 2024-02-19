use napi::bindgen_prelude::Promise;
use napi::tokio::runtime::Handle;
use nodejs_resolver::{Options as ResolverOptions, ResolveResult, Resolver};
use std::sync::Arc;
use std::{
  fmt::Debug,
  fs, io,
  panic::{RefUnwindSafe, UnwindSafe},
  path::{Path, PathBuf},
};
use sugar_path::SugarPath;

use crate::bindings::{
  GrassImporter, LegacyImporterThis, LegacySassImportResult, LegacySassImporter,
  SassCanonicalizeContext, SassImporter, SassImporterResult,
};

pub struct ProxyImporter {
  importers: Vec<SassImporter>,
  legacy_importers: Vec<LegacySassImporter>,
  handle: Handle,
  file: Option<PathBuf>,
  pwd: PathBuf,
  sass_file_resolver: Arc<Resolver>,
}

impl ProxyImporter {
  fn new_sass_file_resolver() -> Resolver {
    Resolver::new(ResolverOptions {
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
    })
  }

  pub fn from_sass_options(
    handle: Handle,
    importers: Option<Vec<SassImporter>>,
    file: Option<PathBuf>,
  ) -> Option<Self> {
    importers.map(|i| Self {
      importers: i,
      legacy_importers: vec![],
      handle,
      file,
      pwd: std::env::current_dir().unwrap(),
      sass_file_resolver: Arc::new(Self::new_sass_file_resolver()),
    })
  }

  pub fn from_legacy_sass_options(
    handle: Handle,
    importer: Option<Vec<LegacySassImporter>>,
    file: Option<PathBuf>,
  ) -> Option<Self> {
    importer.map(|i| Self {
      importers: vec![],
      legacy_importers: i,
      handle,
      file,
      pwd: std::env::current_dir().unwrap(),
      sass_file_resolver: Arc::new(Self::new_sass_file_resolver()),
    })
  }

  pub fn resolve_path_with_ext(&self, p: &str) -> Option<PathBuf> {
    if let Ok(ResolveResult::Resource(resource)) = self.sass_file_resolver.resolve(&self.pwd, p) {
      return Some(resource.path);
    }
    None
  }
}

impl Debug for ProxyImporter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SassImporter")
      .field("importers", &self.importers)
      .finish()
  }
}

impl grass::Fs for ProxyImporter {
  fn is_dir(&self, path: &Path) -> bool {
    path.is_dir()
  }

  fn is_file(&self, path: &Path) -> bool {
    path.is_file()
  }

  fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
    for im in &self.importers {
      if let Some(load) = &im.load {
        match self.handle.block_on(async {
          load
            .call_async::<Promise<Option<SassImporterResult>>>(path.to_string_lossy().to_string())
            .await?
            .await
        }) {
          Ok(res) => {
            if let Some(SassImporterResult { contents, .. }) = res {
              return Ok(contents.into_bytes());
            }
          }
          Err(e) => {
            return Err(io::Error::new(io::ErrorKind::Other, e));
          }
        };
      }
    }

    fs::read(path)
  }

  fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
    dunce::canonicalize(path.normalize())
  }
}

impl RefUnwindSafe for ProxyImporter {}
unsafe impl Send for ProxyImporter {}
unsafe impl Sync for ProxyImporter {}

impl Unpin for ProxyImporter {}

impl UnwindSafe for ProxyImporter {}

impl grass::CustomImporter for ProxyImporter {
  #[tracing::instrument]
  fn find_import(
    &self,
    current_path: &Path,
    import_path: &Path,
    _load_paths: &[PathBuf],
  ) -> Option<PathBuf> {
    let current_path = if current_path.to_string_lossy().eq("stdin") {
      self.file.as_ref().unwrap_or(&self.pwd).clone()
    } else {
      current_path.to_path_buf()
    };
    for legacy_importer in &self.legacy_importers {
      match self.handle.block_on(async {
        legacy_importer
          .call_async::<Promise<Option<LegacySassImportResult>>>((
            LegacyImporterThis { from_import: true },
            import_path.to_string_lossy().to_string(),
            Some(current_path.to_string_lossy().to_string()),
          ))
          .await?
          .await
      }) {
        Ok(res) => {
          if let Some(LegacySassImportResult { file: Some(p), .. }) = res {
            let p_with_ext = self.resolve_path_with_ext(&p);
            if p_with_ext.is_some() {
              return p_with_ext;
            }
          }
        }
        Err(_) => {
          return None;
        }
      }
    }
    for importer in &self.importers {
      if let Some(find_file_url) = &importer.find_file_url {
        if let Ok(res) = self.handle.block_on(async {
          find_file_url
            .call_async::<Promise<Option<String>>>((
              import_path.to_string_lossy().to_string(),
              Some(SassCanonicalizeContext {
                containing_url: Some(current_path.to_string_lossy().to_string()),
                from_import: true,
              }),
            ))
            .await?
            .await
        }) {
          if let Some(p) = res {
            let p_with_ext = self.resolve_path_with_ext(&p);
            if p_with_ext.is_some() {
              return p_with_ext;
            }
          }
        } else {
          return None;
        }
      }
      if let Some(canonicalize) = &importer.canonicalize {
        if let Ok(res) = self.handle.block_on(async {
          canonicalize
            .call_async::<Promise<Option<String>>>((
              import_path.to_string_lossy().to_string(),
              Some(SassCanonicalizeContext {
                containing_url: Some(current_path.to_string_lossy().to_string()),
                from_import: true,
              }),
            ))
            .await?
            .await
        }) {
          if let Some(p) = res {
            let p_with_ext = self.resolve_path_with_ext(&p);
            if p_with_ext.is_some() {
              return p_with_ext;
            }
          }
        } else {
          return None;
        }
      }
    }
    None
  }
}

impl GrassImporter for ProxyImporter {}
