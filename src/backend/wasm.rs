use wasmi::{Engine, Linker, Module, Store, Caller};
use std::collections::HashMap;

pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    pub fn new() -> Self {
        let engine = Engine::default();
        Self { engine }
    }

    pub fn execute(
        &self,
        wasm_bytes: &[u8],
        method: &str,
        params: &[u8],
        state: &HashMap<String, Vec<u8>>,
    ) -> Result<HashMap<String, Vec<u8>>, String> {
        let mut store_data = StoreData {
            state: state.clone(),
            params: params.to_vec(),
            result: Vec::new(),
            error: None,
        };

        let mut store = Store::new(&self.engine, store_data);
        let mut linker = Linker::new(&self.engine);

        // Host functions
        // db_get(key_ptr, key_len, value_ptr) -> value_len
        linker.func_wrap("env", "db_get", |mut caller: Caller<'_, StoreData>, key_ptr: i32, key_len: i32, value_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return -1, // Error
            };
            
            let mut key_buf = vec![0u8; key_len as usize];
            if memory.read(&caller, key_ptr as usize, &mut key_buf).is_err() {
                 return -1;
            }
            let key = String::from_utf8_lossy(&key_buf).to_string();
            
            // Clone the value to drop the borrow on caller
            let value = if let Some(val) = caller.data().state.get(&key) {
                Some(val.clone())
            } else {
                None
            };

            if let Some(val) = value {
                if val.len() > 1024 { return -2; } 
                if memory.write(&mut caller, value_ptr as usize, &val).is_err() {
                    return -1;
                }
                val.len() as i32
            } else {
                0
            }
        }).map_err(|e| e.to_string())?;

        // db_set(key_ptr, key_len, value_ptr, value_len)
        linker.func_wrap("env", "db_set", |mut caller: Caller<'_, StoreData>, key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return, 
            };
            
            let mut key_buf = vec![0u8; key_len as usize];
            if memory.read(&caller, key_ptr as usize, &mut key_buf).is_err() { return; }
            let key = String::from_utf8_lossy(&key_buf).to_string();

            let mut value_buf = vec![0u8; value_len as usize];
            if memory.read(&caller, value_ptr as usize, &mut value_buf).is_err() { return; }
            
            caller.data_mut().state.insert(key, value_buf);
        }).map_err(|e| e.to_string())?;

        // response_write(ptr, len)
        linker.func_wrap("env", "response_write", |mut caller: Caller<'_, StoreData>, ptr: i32, len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return,
            };
            let mut buf = vec![0u8; len as usize];
            if memory.read(&caller, ptr as usize, &mut buf).is_err() { return; }
            caller.data_mut().result.extend_from_slice(&buf);
        }).map_err(|e| e.to_string())?;

        // db_remove(key_ptr, key_len)
        linker.func_wrap("env", "db_remove", |mut caller: Caller<'_, StoreData>, key_ptr: i32, key_len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return,
            };
            let mut key_buf = vec![0u8; key_len as usize];
            if memory.read(&caller, key_ptr as usize, &mut key_buf).is_err() { return; }
            let key = String::from_utf8_lossy(&key_buf).to_string();
            caller.data_mut().state.remove(&key);
        }).map_err(|e| e.to_string())?;
        
        // get_params(ptr) -> len
         linker.func_wrap("env", "get_params", |mut caller: Caller<'_, StoreData>, ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return -1,
            };
            let params = caller.data().params.clone();
             if memory.write(&mut caller, ptr as usize, &params).is_err() {
                return -1;
            }
            params.len() as i32
        }).map_err(|e| e.to_string())?;

        let module = Module::new(&self.engine, wasm_bytes).map_err(|e| e.to_string())?;
        let instance = linker.instantiate(&mut store, &module).map_err(|e| e.to_string())?.start(&mut store).map_err(|e| e.to_string())?;

        let run = instance.get_typed_func::<(), ()>(&store, method).map_err(|e| e.to_string())?;
        run.call(&mut store, ()).map_err(|e| e.to_string())?;

        Ok(store.into_data().state)
    }

    pub fn render(
        &self,
        wasm_bytes: &[u8],
        params: &[u8],
        state: &HashMap<String, Vec<u8>>,
    ) -> Result<String, String> {
        let mut store_data = StoreData {
            state: state.clone(),
            params: params.to_vec(),
            result: Vec::new(),
            error: None,
        };

        let mut store = Store::new(&self.engine, store_data);
        let mut linker = Linker::new(&self.engine);

        // Host functions (same as execute for DB access if needed)
        // db_get(key_ptr, key_len, value_ptr) -> value_len
        linker.func_wrap("env", "db_get", |mut caller: Caller<'_, StoreData>, key_ptr: i32, key_len: i32, value_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return -1,
            };
            
            let mut key_buf = vec![0u8; key_len as usize];
            if memory.read(&caller, key_ptr as usize, &mut key_buf).is_err() {
                 return -1;
            }
            let key = String::from_utf8_lossy(&key_buf).to_string();
            
            let value = if let Some(val) = caller.data().state.get(&key) {
                Some(val.clone())
            } else {
                None
            };

            if let Some(val) = value {
                if val.len() > 1024 { return -2; } 
                if memory.write(&mut caller, value_ptr as usize, &val).is_err() {
                    return -1;
                }
                val.len() as i32
            } else {
                0
            }
        }).map_err(|e| e.to_string())?;

        // response_write(ptr, len)
        linker.func_wrap("env", "response_write", |mut caller: Caller<'_, StoreData>, ptr: i32, len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return,
            };
            let mut buf = vec![0u8; len as usize];
            if memory.read(&caller, ptr as usize, &mut buf).is_err() { return; }
            caller.data_mut().result.extend_from_slice(&buf);
        }).map_err(|e| e.to_string())?;

         // get_params(ptr) -> len
         linker.func_wrap("env", "get_params", |mut caller: Caller<'_, StoreData>, ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmi::Extern::Memory(m)) => m,
                _ => return -1,
            };
            let params = caller.data().params.clone();
             if memory.write(&mut caller, ptr as usize, &params).is_err() {
                return -1;
            }
            params.len() as i32
        }).map_err(|e| e.to_string())?;

        let module = Module::new(&self.engine, wasm_bytes).map_err(|e| e.to_string())?;
        let instance = linker.instantiate(&mut store, &module).map_err(|e| e.to_string())?.start(&mut store).map_err(|e| e.to_string())?;

        // Expect a "render" export
        let run = instance.get_typed_func::<(), ()>(&store, "render").map_err(|e| e.to_string())?;
        run.call(&mut store, ()).map_err(|e| e.to_string())?;

        let result = store.into_data().result;
        String::from_utf8(result).map_err(|e| e.to_string())
    }
}

struct StoreData {
    state: HashMap<String, Vec<u8>>,
    params: Vec<u8>,
    result: Vec<u8>,
    error: Option<String>,
}
