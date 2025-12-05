use std::collections::HashMap;
use crate::backend::dag::{DagNode, DagPayload};
use crate::backend::wasm::WasmRuntime;
use serde_json;
use hex;

pub struct VM;

impl VM {
    /// Calculate the current state of a contract by replaying all calls against the initial state.
    pub fn calculate_contract_state(init_state: &str, code: &str, calls: &[DagNode]) -> String {
        // WASM Detection: Check for magic bytes or hex-encoded magic bytes
        let wasm_bytes = if code.starts_with("\0asm") {
            code.as_bytes().to_vec()
        } else if code.len() > 8 {
             if let Ok(bytes) = hex::decode(&code) {
                 if bytes.starts_with(b"\0asm") {
                     bytes
                 } else {
                     Vec::new()
                 }
             } else {
                 Vec::new()
             }
        } else {
            Vec::new()
        };

        let is_wasm = !wasm_bytes.is_empty();

        if is_wasm {
            println!("Executing WASM contract");
            
            let runtime = WasmRuntime::new();
            let mut state: HashMap<String, Vec<u8>> = serde_json::from_str(init_state).unwrap_or_default();
            
            // Try to parse init_state as simple JSON map first if the above failed or produced empty for non-byte map
            if state.is_empty() {
                 if let Ok(init_map) = serde_json::from_str::<HashMap<String, String>>(init_state) {
                     for (k, v) in init_map {
                         state.insert(k, v.into_bytes());
                     }
                }
            }

            for call_node in calls {
                 if let DagPayload::ContractCall(call) = &call_node.payload {
                     let params = call.params.as_bytes();
                     match runtime.execute(&wasm_bytes, &call.method, params, &state) {
                         Ok(new_state) => state = new_state,
                         Err(e) => println!("WASM execution error: {}", e),
                     }
                 }
            }
            
            // Convert back to JSON for UI
            let mut json_map = HashMap::new();
            for (k, v) in state {
                json_map.insert(k, String::from_utf8_lossy(&v).to_string());
            }
            serde_json::to_string_pretty(&json_map).unwrap_or("{}".to_string())
        } else {
            // KV Logic (Legacy / Default)
             let mut state_val: serde_json::Value = serde_json::from_str(init_state).unwrap_or(serde_json::json!({}));

            for call_node in calls {
                if let DagPayload::ContractCall(call) = &call_node.payload {
                     if call.method == "set" {
                         if let Ok(params) = serde_json::from_str::<serde_json::Value>(&call.params) {
                             if let (Some(k), Some(v)) = (params.get("key").and_then(|s| s.as_str()), params.get("value")) {
                                 if let Some(obj) = state_val.as_object_mut() {
                                     obj.insert(k.to_string(), v.clone());
                                 }
                             }
                         }
                     } else if call.method == "delete" {
                         if let Ok(params) = serde_json::from_str::<serde_json::Value>(&call.params) {
                             if let Some(k) = params.get("key").and_then(|s| s.as_str()) {
                                 if let Some(obj) = state_val.as_object_mut() {
                                     obj.remove(k);
                                 }
                             }
                         }
                     }
                }
            }
            serde_json::to_string_pretty(&state_val).unwrap_or("{}".to_string())
        }
    }

    /// Render a web page content, processing WASM if detected.
    pub fn render_web_page(content: &str) -> String {
         // Check if content is WASM (Hex encoded or raw string starting with \0asm)
         let wasm_bytes = if content.starts_with("\0asm") {
             content.as_bytes().to_vec()
         } else if content.len() > 8 {
              if let Ok(bytes) = hex::decode(&content) {
                  if bytes.starts_with(b"\0asm") {
                      bytes
                  } else {
                      Vec::new()
                  }
              } else {
                  Vec::new()
              }
         } else {
             Vec::new()
         };

         if !wasm_bytes.is_empty() {
             println!("Detected WASM web page, rendering...");
             let runtime = WasmRuntime::new();
             match runtime.render(&wasm_bytes, &[], &HashMap::new()) {
                 Ok(html) => html,
                 Err(e) => format!("<h1>Error rendering page</h1><pre>{}</pre>", e),
             }
         } else {
             content.to_string()
         }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::dag::{DagNode, DagPayload, ContractCallPayload};
    use libp2p::identity::Keypair;

    fn create_dummy_call(method: &str, params: &str) -> DagNode {
        let keypair = Keypair::generate_ed25519();
        let payload = DagPayload::ContractCall(ContractCallPayload {
            contract_id: "test".to_string(),
            method: method.to_string(),
            params: params.to_string(),
        });
        DagNode::new("contract_call:v1".to_string(), payload, vec![], &keypair, 0).unwrap()
    }

    #[test]
    fn test_kv_contract_set_delete() {
        let init_state = r#"{"count": "0"}"#;
        let code = ""; // Empty code means KV contract

        let call1 = create_dummy_call("set", r#"{"key": "test_key", "value": "test_value"}"#);
        let call2 = create_dummy_call("set", r#"{"key": "another", "value": "123"}"#);
        let call3 = create_dummy_call("delete", r#"{"key": "test_key"}"#);

        let calls = vec![call1, call2, call3];

        let state_json = VM::calculate_contract_state(init_state, code, &calls);
        let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();

        assert_eq!(state["count"], "0");
        assert_eq!(state["another"], "123");
        assert!(state.get("test_key").is_none());
    }

    #[test]
    fn test_render_static_web_page() {
        let content = "<h1>Hello</h1>";
        let rendered = VM::render_web_page(content);
        assert_eq!(rendered, "<h1>Hello</h1>");
    }

    // Note: Testing WASM execution requires a valid WASM binary which is hard to construct inline.
    // We rely on the KV test for VM structure and assume WasmRuntime works as tested in its own module/integration tests if any.
}
