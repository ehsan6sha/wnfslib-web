use libipld::Cid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use js_sys::{Reflect, Uint8Array, Function, Promise};
use log::{trace, Level};
use console_log;
use console_error_panic_hook;
use anyhow::{Result, Error};
use serde_wasm_bindgen;
use wasm_bindgen_futures::js_sys;
use wnfsutils::blockstore::{FFIFriendlyBlockStore, FFIStore};
use wnfsutils::private_forest::PrivateDirectoryHelper;
use wasm_bindgen_futures::JsFuture;
use futures_util::TryFutureExt;
use serde::Serialize;

#[derive(Serialize)]
struct PrivateDirectoryHelperResult {
    forest_cid: String,
    root_dir_cid: String,
}

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Trace).expect("Error initializing logger");
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct JSStore {
    js_client: JsValue,
}

#[wasm_bindgen]
impl JSStore {
    #[wasm_bindgen(constructor)]
    pub fn new(js_client: JsValue) -> Self {
        Self { js_client }
    }

    #[wasm_bindgen]
    pub async fn get_block(&self, cid: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        trace!("**********************get_block started**************");

        // Convert CID to Uint8Array
        let cid_js_array = Uint8Array::from(cid.as_slice());

        // Get the "get" method from js_client
        let binding = Reflect::get(&self.js_client, &JsValue::from_str("get"))
            .map_err(|e| JsValue::from_str(&format!("Failed to get 'get' method: {:?}", e)))?;
        let get_fn = binding
            .dyn_ref::<Function>()
            .ok_or_else(|| JsValue::from_str("Expected 'get' to be a JavaScript function"))?;

        // Call the "get" method (returns a Promise)
        let promise_value = get_fn
            .call1(&self.js_client, &cid_js_array.into())
            .map_err(|e| JsValue::from_str(&format!("Failed to call 'get': {:?}", e)))?;
        trace!("Raw result from JS get method (Promise): {:?}", promise_value);

        // Convert JsValue to js_sys::Promise
        let promise = promise_value.dyn_into::<js_sys::Promise>().map_err(|e| {
            JsValue::from_str(&format!("Failed to convert JsValue to Promise: {:?}", e))
        })?;

        // Await the Promise and resolve it into a JsValue
        let result = JsFuture::from(promise).await?;
        trace!("Resolved result from JS Promise: {:?}", result);

        // Clone result before calling dyn_into to avoid ownership issues
        let data_js_array = result.clone().dyn_into::<Uint8Array>().map_err(|e| {
            JsValue::from_str(&format!(
                "Failed to convert result to Uint8Array: {:?}, error: {:?}",
                result, e
            ))
        })?;

        // Check for empty data
        if data_js_array.length() == 0 {
            return Err(JsValue::from_str("Block data is empty"));
        }

        // Convert Uint8Array to Vec<u8>
        let data = data_js_array.to_vec();
        trace!(
            "**********************get_block Retrieved bytes for CID {:?}: {:?}",
            cid,
            data
        );
        trace!("**********************get_block finished**************");
        Ok(data)
    }

    #[wasm_bindgen]
    pub fn put_block(&self, cid: Vec<u8>, bytes: Vec<u8>) -> Result<(), JsValue> {
        trace!("**********************put_block started**************");

        // Convert CID and bytes to Uint8Array
        let cid_js_array = Uint8Array::from(cid.as_slice());
        let bytes_js_array = Uint8Array::from(bytes.as_slice());

        // Get the "put" method from js_client
        let binding = Reflect::get(&self.js_client, &JsValue::from_str("put"))
            .map_err(|e| JsValue::from_str(&format!("Failed to get 'put' method: {:?}", e)))?;
        let put_fn = binding
            .dyn_ref::<Function>()
            .ok_or_else(|| JsValue::from_str("Expected 'put' to be a JavaScript function"))?;

        // Call the "put" method
        put_fn
            .call2(&self.js_client, &cid_js_array.into(), &bytes_js_array.into())
            .map_err(|e| JsValue::from_str(&format!("Failed to call 'put': {:?}", e)))?;

        trace!("**********************put_block Put bytes for CID {:?}:>>>>>>>>>>>>>> {:?}", cid, bytes);
        trace!("**********************put_block finished**************");
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl<'a> FFIStore<'a> for JSStore {
    async fn get_block(&self, cid: Vec<u8>) -> Result<Vec<u8>> {
        self.get_block(cid)
            .await
            .map_err(|e| Error::msg(format!("{:?}", e)))
    }

    async fn put_block(&self, cid: Vec<u8>, bytes: Vec<u8>) -> Result<()> {
        self.put_block(cid, bytes)
            .map_err(|e| Error::msg(format!("{:?}", e)))
    }
}

#[wasm_bindgen]
pub async fn init_native(js_client: JsValue, wnfs_key: &[u8]) -> Result<JsValue, JsValue> {
    trace!("**********************init_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    match PrivateDirectoryHelper::init_async(&mut block_store, wnfs_key.to_vec()).await {
        Ok((_, _, cid)) => {
            trace!("init_native succeeded");
            serde_wasm_bindgen::to_value(&cid).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        Err(err) => {
            trace!("init_native failed: {:?}", err);
            Err(JsValue::from_str(&err.to_string()))
        }
    }
}

#[wasm_bindgen]
pub async fn mkdir_native(
    js_client: JsValue,
    cid: &[u8],
    path_segments: &str,
) -> Result<JsValue, JsValue> {
    trace!("**********************mkdir_native started**************");
    trace!("**********************mkdir_native received CID: {:?}", cid);

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid).map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare path segments
        let path_segments: Vec<String> = path_segments.split('/').map(String::from).collect();

        // Perform mkdir operation asynchronously
        match helper.mkdir_async(&path_segments).await {
            Ok(new_cid) => {
                trace!("**********************mkdir_native finished**************");
                serde_wasm_bindgen::to_value(&new_cid)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in mkdir_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in mkdir_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}

#[wasm_bindgen]
pub async fn ls_native(
    js_client: JsValue,
    cid: &[u8],
    path_segments: &str,
) -> Result<JsValue, JsValue> {
    trace!("**********************ls_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid).map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare path segments
        let path_segments: Vec<String> = path_segments.split('/').map(String::from).collect();

        // Perform ls operation asynchronously
        match helper.ls_files_async(&path_segments).await {
            Ok(ls_result) => {
                trace!("**********************ls_native finished**************");
                serde_wasm_bindgen::to_value(&ls_result)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in ls_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in ls_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}

#[wasm_bindgen]
pub async fn load_with_wnfs_key_native(
    js_client: JsValue,
    forest_cid: &[u8],
    wnfs_key: &[u8],
) -> Result<JsValue, JsValue> {
    trace!("**********************load_with_wnfs_key_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(forest_cid)
        .map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Call the async method
    match PrivateDirectoryHelper::load_with_wnfs_key_async(&mut block_store, cid, wnfs_key.to_vec()).await {
        Ok(helper) => {
            trace!("load_with_wnfs_key_native succeeded");

            // Construct the result struct
            let result = PrivateDirectoryHelperResult {
                forest_cid: cid.to_string(),
                root_dir_cid: cid.to_string(),
            };

            // Serialize the result struct into a JsValue
            serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        Err(err) => {
            trace!("wnfsError in load_with_wnfs_key_native: {:?}", err);
            Err(JsValue::from_str(&err.to_string()))
        }
    }
}

#[wasm_bindgen]
pub async fn write_file_native(
    js_client: JsValue,
    cid: &[u8],
    path_segments: &str,
    content: Vec<u8>,
    modification_time_seconds: i64,
) -> Result<JsValue, JsValue> {
    trace!("**********************write_file_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid)
        .map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare path segments
        let path_segments: Vec<String> = path_segments.split('/').map(String::from).collect();

        // Perform write file operation asynchronously
        match helper.write_file_async(&path_segments, content, modification_time_seconds).await {
            Ok(new_cid) => {
                trace!("**********************write_file_native finished**************");
                serde_wasm_bindgen::to_value(&new_cid)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in write_file_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in write_file_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}

#[wasm_bindgen]
pub async fn read_file_native(
    js_client: JsValue,
    cid: &[u8],
    path_segments: &str,
) -> Result<JsValue, JsValue> {
    trace!("**********************read_file_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid)
        .map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare path segments
        let path_segments: Vec<String> = path_segments.split('/').map(String::from).collect();

        // Perform read file operation asynchronously
        match helper.read_file_async(&path_segments).await {
            Ok(file_content) => {
                trace!("**********************read_file_native finished**************");
                serde_wasm_bindgen::to_value(&file_content)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in read_file_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in read_file_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}

#[wasm_bindgen]
pub async fn mv_native(
    js_client: JsValue,
    cid: &[u8],
    source_path_segments: &str,
    target_path_segments: &str,
) -> Result<JsValue, JsValue> {
    trace!("**********************mv_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid)
        .map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare source and target path segments
        let source_path_segments: Vec<String> =
            source_path_segments.split('/').map(String::from).collect();
        let target_path_segments: Vec<String> =
            target_path_segments.split('/').map(String::from).collect();

        // Perform move operation asynchronously
        match helper.mv_async(&source_path_segments, &target_path_segments).await {
            Ok(new_cid) => {
                trace!("**********************mv_native finished**************");
                serde_wasm_bindgen::to_value(&new_cid)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in mv_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in mv_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}

#[wasm_bindgen]
pub async fn cp_native(
    js_client: JsValue,
    cid: &[u8],
    source_path_segments: &str,
    target_path_segments: &str,
) -> Result<JsValue, JsValue> {
    trace!("**********************cp_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid)
        .map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare source and target path segments
        let source_path_segments: Vec<String> =
            source_path_segments.split('/').map(String::from).collect();
        let target_path_segments: Vec<String> =
            target_path_segments.split('/').map(String::from).collect();

        // Perform copy operation asynchronously
        match helper.cp_async(&source_path_segments, &target_path_segments).await {
            Ok(new_cid) => {
                trace!("**********************cp_native finished**************");
                serde_wasm_bindgen::to_value(&new_cid)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in cp_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in cp_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}

#[wasm_bindgen]
pub async fn rm_native(
    js_client: JsValue,
    cid: &[u8],
    path_segments: &str,
) -> Result<JsValue, JsValue> {
    trace!("**********************rm_native started**************");

    // Create JSStore instance
    let store = JSStore::new(js_client);
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));

    // Deserialize the CID
    let cid = Cid::try_from(cid)
        .map_err(|e| JsValue::from_str(&format!("Invalid CID: {:?}", e)))?;

    // Reload the private directory helper asynchronously
    let helper_res = PrivateDirectoryHelper::reload_async(&mut block_store, cid).await;

    if let Ok(mut helper) = helper_res {
        // Prepare path segments
        let path_segments: Vec<String> = path_segments.split('/').map(String::from).collect();

        // Perform remove operation asynchronously
        match helper.rm_async(&path_segments).await {
            Ok(new_cid) => {
                trace!("**********************rm_native finished**************");
                serde_wasm_bindgen::to_value(&new_cid)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => {
                trace!("wnfsError in rm_native: {:?}", err);
                Err(JsValue::from_str(&err.to_string()))
            }
        }
    } else {
        let err = helper_res.err().unwrap();
        trace!("wnfsError in rm_native (reload): {:?}", err);
        Err(JsValue::from_str(&err.to_string()))
    }
}