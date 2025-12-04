use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use tesser_wasm::{
    host::{ComponentBindings, DecimalValue, WasiSide, WasiTick},
    PluginSide, PluginTick,
};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{command::sync::add_to_linker, WasiCtx, WasiCtxBuilder, WasiView};

/// Runtime responsible for loading, caching, and instantiating WASM plugins.
#[derive(Clone)]
pub struct WasmPluginEngine {
    engine: Arc<Engine>,
    cache: Arc<Mutex<HashMap<PathBuf, CachedComponent>>>,
    plugins_dir: PathBuf,
}

struct CachedComponent {
    component: Arc<Component>,
    modified: SystemTime,
}

impl WasmPluginEngine {
    /// Create a new engine rooted at the provided plugin directory.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(false);
        let engine = Engine::new(&config)?;
        Ok(Self {
            engine: Arc::new(engine),
            cache: Arc::new(Mutex::new(HashMap::new())),
            plugins_dir: dir.into(),
        })
    }

    fn resolve_path(&self, raw: &str) -> PathBuf {
        let trimmed = raw.trim();
        let candidate = Path::new(trimmed);
        let mut joined = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.plugins_dir.join(candidate)
        };
        if joined.extension().is_none() {
            joined.set_extension("wasm");
        }
        joined
    }

    fn load_component(&self, name: &str) -> Result<Arc<Component>> {
        let path = self.resolve_path(name);
        let metadata = fs::metadata(&path)
            .with_context(|| format!("plugin '{}' not found in {}", name, path.display()))?;
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        let canonical = fs::canonicalize(&path).unwrap_or(path.clone());

        let mut cache = self
            .cache
            .lock()
            .map_err(|_| anyhow!("wasm component cache poisoned"))?;
        if let Some(entry) = cache.get(&canonical) {
            if entry.modified >= modified {
                return Ok(entry.component.clone());
            }
        }

        let component = Component::from_file(&self.engine, &canonical).with_context(|| {
            format!(
                "failed to compile plugin '{}' from {}",
                name,
                canonical.display()
            )
        })?;
        let arc = Arc::new(component);
        cache.insert(
            canonical,
            CachedComponent {
                component: arc.clone(),
                modified,
            },
        );
        Ok(arc)
    }

    /// Instantiate a new WASM component for the supplied plugin name.
    pub fn instantiate(&self, name: &str) -> Result<WasmInstance> {
        let component = self.load_component(name)?;
        WasmInstance::new(self.engine.clone(), component)
    }
}

struct PluginStore {
    table: ResourceTable,
    wasi: WasiCtx,
}

impl PluginStore {
    fn new() -> Self {
        let wasi = WasiCtxBuilder::new().build();
        Self {
            table: ResourceTable::new(),
            wasi,
        }
    }
}

impl Default for PluginStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WasiView for PluginStore {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

/// Active plugin instance backed by a wasmtime store.
pub struct WasmInstance {
    store: Store<PluginStore>,
    plugin: ComponentBindings,
}

impl WasmInstance {
    fn new(engine: Arc<Engine>, component: Arc<Component>) -> Result<Self> {
        let mut linker: Linker<PluginStore> = Linker::new(&engine);
        add_to_linker(&mut linker)?;
        let mut store = Store::new(&engine, PluginStore::new());
        let (plugin, _) = ComponentBindings::instantiate(&mut store, component.as_ref(), &linker)?;
        Ok(Self { store, plugin })
    }

    pub fn call_init(&mut self, payload: &str) -> Result<String> {
        match self
            .plugin
            .call_init(&mut self.store, payload)
            .context("plugin init failed")?
        {
            Ok(value) => Ok(value),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn call_on_tick(&mut self, tick: &PluginTick) -> Result<String> {
        let wasi_tick = Self::convert_tick(tick);
        self.plugin
            .call_on_tick(&mut self.store, &wasi_tick)
            .context("plugin on_tick failed")
    }

    pub fn call_on_fill(&mut self, payload: &str) -> Result<String> {
        self.plugin
            .call_on_fill(&mut self.store, payload)
            .context("plugin on_fill failed")
    }

    pub fn call_on_timer(&mut self) -> Result<String> {
        self.plugin
            .call_on_timer(&mut self.store)
            .context("plugin on_timer failed")
    }

    pub fn call_snapshot(&mut self) -> Result<String> {
        self.plugin
            .call_snapshot(&mut self.store)
            .context("plugin snapshot failed")
    }

    pub fn call_restore(&mut self, payload: &str) -> Result<()> {
        self.plugin
            .call_restore(&mut self.store, payload)
            .context("plugin restore failed")
    }

    fn convert_tick(tick: &PluginTick) -> WasiTick {
        WasiTick {
            symbol: tick.symbol.clone(),
            price: DecimalValue {
                value: tick.price.to_string(),
            },
            size: DecimalValue {
                value: tick.size.to_string(),
            },
            side: match tick.side {
                PluginSide::Buy => WasiSide::Buy,
                PluginSide::Sell => WasiSide::Sell,
            },
            timestamp_ms: tick.timestamp_ms,
        }
    }
}
