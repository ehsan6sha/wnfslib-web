#[cfg(target_arch = "wasm32")]
mod web {
    use libipld::Cid;
    use wnfs::common::Metadata;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::future_to_promise;
    use anyhow::Result;
    use wnfsutils::blockstore::{FFIFriendlyBlockStore, FFIStore};
    use wnfsutils::private_forest::PrivateDirectoryHelper;
    use log::trace;
    // Or import the entire prelude


    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);

        #[wasm_bindgen(js_namespace = console)]
        fn error(s: &str);

        #[wasm_bindgen(js_name = getFromStorage)]
        fn get_from_storage(key: &str) -> js_sys::Promise;

        #[wasm_bindgen(js_name = putToStorage)]
        fn put_to_storage(key: &str, value: &[u8]) -> js_sys::Promise;
    }

    #[derive(Clone)]
    struct WebStore {
        prefix: String,
    }

    impl<'a> WebStore {
        fn new(prefix: String) -> Self {
            Self { prefix }
        }
    }

    impl<'a> FFIStore<'a> for WebStore {
        fn get_block(&self, cid: Vec<u8>) -> Result<Vec<u8>, anyhow::Error> {
            let cid_str = hex::encode(&cid);
            let key = format!("{}{}", self.prefix, cid_str);
            
            // Convert JavaScript Promise to Rust Future
            let promise = get_from_storage(&key);
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            
            // Execute the future and handle the result
            let result: Result<Vec<u8>, anyhow::Error> = futures::executor::block_on(async move {
                let js_value = future.await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
                let array = js_sys::Uint8Array::new(&js_value);
                Ok(array.to_vec())
            });
        
            match result {
                Ok(data) => {
                    trace!("**********************get_block finished**************");
                    Ok(data)
                }
                Err(e) => {
                    trace!("wnfsError get_block: {:?}", e.to_string());
                    Ok(Vec::new())
                }
            }
        }

        fn put_block(&self, cid: Vec<u8>, bytes: Vec<u8>) -> Result<(), anyhow::Error> {
            let cid_str = hex::encode(&cid);
            let key = format!("{}{}", self.prefix, cid_str);
            
            trace!("**********************put_block started**************");
            trace!("**********************put_block cid={:?}", &cid);
            trace!("**********************put_block bytes={:?}", &bytes);
            
            // Convert JavaScript Promise to Rust Future
            let promise = put_to_storage(&key, &bytes);
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            
            // Execute the future and handle the result
            let result: Result<(), anyhow::Error> = futures::executor::block_on(async move {
                future.await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
                Ok(())
            });
        
            match result {
                Ok(_) => {
                    trace!("**********************put_block finished**************");
                    Ok(())
                }
                Err(e) => {
                    trace!("**********************put_block error: {:?}**************", e);
                    Err(e)
                }
            }
        }
    }

    #[wasm_bindgen]
    pub fn init_rust_logger() {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    }

    #[wasm_bindgen]
    pub fn load_with_wnfs_key_native(
        fula_client_prefix: &str,
        wnfs_key: Vec<u8>,
        cid: String,
    ) -> js_sys::Promise {
        let store = WebStore::new(fula_client_prefix.to_string());
        let block_store = &mut FFIFriendlyBlockStore::new(Box::new(store));
        let forest_cid = deserialize_cid(cid);
        let helper_res = PrivateDirectoryHelper::synced_load_with_wnfs_key(block_store, forest_cid, wnfs_key);
        future_to_promise(async move {
            match helper_res {
                Ok(_) => Ok(JsValue::null()),
                Err(msg) => Err(JsValue::from_str(&msg.to_string())),
            }
        })
    }

    #[wasm_bindgen]
    pub fn init_native(
        fula_client_prefix: &str,
        wnfs_key: Vec<u8>,
    ) -> js_sys::Promise {
        let store = WebStore::new(fula_client_prefix.to_string());
        let block_store = &mut FFIFriendlyBlockStore::new(Box::new(store));
        let helper_res = PrivateDirectoryHelper::synced_init(block_store, wnfs_key);
        future_to_promise(async move {
            match helper_res {
                Ok((_, _, cid)) => Ok(serde_wasm_bindgen::to_value(&cid).unwrap()),
                Err(msg) => Err(JsValue::from_str(&msg.to_string())),
            }
        })
    }

    #[wasm_bindgen]
    pub fn write_file_native(
        fula_client_prefix: &str,
        cid: String,
        path_segments: String,
        content: Vec<u8>,
    ) -> js_sys::Promise {
        trace!("**********************writeFileNative started**************");
        let store = WebStore::new(fula_client_prefix.to_string());
        let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
        let forest_cid = deserialize_cid(cid);
        
        future_to_promise(async move {
            match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
                Ok(mut helper) => {
                    let path_segments = prepare_path_segments(path_segments);
                    let segments: Vec<String> = serde_wasm_bindgen::from_value(path_segments).unwrap();
                    
                    match helper.synced_write_file(&segments, content, 0) {
                        Ok(new_cid) => {
                            trace!("**********************writeFileNative finished**************");
                            Ok(serde_wasm_bindgen::to_value(&new_cid).unwrap())
                        }
                        Err(msg) => {
                            trace!("wnfsError in write_file_native: {:?}", msg);
                            Err(JsValue::from_str(&msg.to_string()))
                        }
                    }
                }
                Err(msg) => {
                    trace!("wnfsError in write_file_native: {:?}", msg);
                    Err(JsValue::from_str(&msg.to_string()))
                }
            }
        })
    }

    #[wasm_bindgen]
    pub fn read_file_native(
        fula_client_prefix: &str,
        cid: String,
        path_segments: String,
    ) -> js_sys::Promise {
        let store = WebStore::new(fula_client_prefix.to_string());
        let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
        let forest_cid = deserialize_cid(cid);
        
        future_to_promise(async move {
            match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
                Ok(mut helper) => {
                    let segments = prepare_path_segments(path_segments);
                    let path_vec: Vec<String> = serde_wasm_bindgen::from_value(segments)?;
                    
                    match helper.synced_read_file(&path_vec) {
                        Ok(content) => {
                            let array = js_sys::Uint8Array::from(content.as_slice());
                            Ok(array.into())
                        }
                        Err(msg) => Err(JsValue::from_str(&msg.to_string()))
                    }
                }
                Err(msg) => Err(JsValue::from_str(&msg.to_string()))
            }
        })
    }

    #[wasm_bindgen]
    pub fn mkdir_native(
        fula_client_prefix: &str,
        cid: String,
        path_segments: String,
    ) -> js_sys::Promise {
        trace!("**********************mkDirNative started**************");
        let store = WebStore::new(fula_client_prefix.to_string());
        let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
        let forest_cid = deserialize_cid(cid);
        
        future_to_promise(async move {
            match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
                Ok(mut helper) => {
                    let segments = prepare_path_segments(path_segments);
                    let path_vec: Vec<String> = serde_wasm_bindgen::from_value(segments)?;
                    
                    match helper.synced_mkdir(&path_vec) {
                        Ok(new_cid) => {
                            trace!("**********************mkDirNative finished**************");
                            Ok(serde_wasm_bindgen::to_value(&new_cid).unwrap())
                        }
                        Err(msg) => {
                            trace!("wnfsError in mkdir_native: {:?}", msg);
                            Err(JsValue::from_str(&msg.to_string()))
                        }
                    }
                }
                Err(msg) => {
                    trace!("wnfsError in mkdir_native: {:?}", msg);
                    Err(JsValue::from_str(&msg.to_string()))
                }
            }
        })
    }

    #[wasm_bindgen]
pub fn mv_native(
    fula_client_prefix: &str,
    cid: String,
    source_path_segments: String,
    target_path_segments: String,
) -> js_sys::Promise {
    trace!("**********************mvNative started**************");
    let store = WebStore::new(fula_client_prefix.to_string());
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
    let forest_cid = deserialize_cid(cid);
    
    future_to_promise(async move {
        match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
            Ok(mut helper) => {
                let source_segments = prepare_path_segments(source_path_segments);
                let target_segments = prepare_path_segments(target_path_segments);
                
                let source_vec: Vec<String> = serde_wasm_bindgen::from_value(source_segments)?;
                let target_vec: Vec<String> = serde_wasm_bindgen::from_value(target_segments)?;
                
                match helper.synced_mv(&source_vec, &target_vec) {
                    Ok(new_cid) => {
                        trace!("**********************mvNative finished**************");
                        Ok(serde_wasm_bindgen::to_value(&new_cid).unwrap())
                    }
                    Err(msg) => {
                        trace!("wnfsError in mv_native: {:?}", msg);
                        Err(JsValue::from_str(&msg.to_string()))
                    }
                }
            }
            Err(msg) => {
                trace!("wnfsError in mv_native: {:?}", msg);
                Err(JsValue::from_str(&msg.to_string()))
            }
        }
    })
}

#[wasm_bindgen]
pub fn cp_native(
    fula_client_prefix: &str,
    cid: String,
    source_path_segments: String,
    target_path_segments: String,
) -> js_sys::Promise {
    trace!("**********************cpNative started**************");
    let store = WebStore::new(fula_client_prefix.to_string());
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
    let forest_cid = deserialize_cid(cid);
    
    future_to_promise(async move {
        match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
            Ok(mut helper) => {
                let source_segments = prepare_path_segments(source_path_segments);
                let target_segments = prepare_path_segments(target_path_segments);
                
                let source_vec: Vec<String> = serde_wasm_bindgen::from_value(source_segments)?;
                let target_vec: Vec<String> = serde_wasm_bindgen::from_value(target_segments)?;
                
                match helper.synced_cp(&source_vec, &target_vec) {
                    Ok(new_cid) => {
                        trace!("**********************cpNative finished**************");
                        Ok(serde_wasm_bindgen::to_value(&new_cid).unwrap())
                    }
                    Err(msg) => {
                        trace!("wnfsError in cp_native: {:?}", msg);
                        Err(JsValue::from_str(&msg.to_string()))
                    }
                }
            }
            Err(msg) => {
                trace!("wnfsError in cp_native: {:?}", msg);
                Err(JsValue::from_str(&msg.to_string()))
            }
        }
    })
}

#[wasm_bindgen]
pub fn rm_native(
    fula_client_prefix: &str,
    cid: String,
    path_segments: String,
) -> js_sys::Promise {
    trace!("**********************rmNative started**************");
    let store = WebStore::new(fula_client_prefix.to_string());
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
    let forest_cid = deserialize_cid(cid);
    
    future_to_promise(async move {
        match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
            Ok(mut helper) => {
                let segments = prepare_path_segments(path_segments);
                let path_vec: Vec<String> = serde_wasm_bindgen::from_value(segments)?;
                
                match helper.synced_rm(&path_vec) {
                    Ok(new_cid) => {
                        trace!("**********************rmNative finished**************");
                        Ok(serde_wasm_bindgen::to_value(&new_cid).unwrap())
                    }
                    Err(msg) => {
                        trace!("wnfsError in rm_native: {:?}", msg);
                        Err(JsValue::from_str(&msg.to_string()))
                    }
                }
            }
            Err(msg) => {
                trace!("wnfsError in rm_native: {:?}", msg);
                Err(JsValue::from_str(&msg.to_string()))
            }
        }
    })
}

#[wasm_bindgen]
pub fn ls_native(
    fula_client_prefix: &str,
    cid: String,
    path_segments: String,
) -> js_sys::Promise {
    trace!("**********************lsNative started**************");
    let store = WebStore::new(fula_client_prefix.to_string());
    let mut block_store = FFIFriendlyBlockStore::new(Box::new(store));
    let forest_cid = deserialize_cid(cid);
    
    future_to_promise(async move {
        match PrivateDirectoryHelper::synced_reload(&mut block_store, forest_cid) {
            Ok(mut helper) => {
                let segments = prepare_path_segments(path_segments);
                let path_vec: Vec<String> = serde_wasm_bindgen::from_value(segments)?;
                
                match helper.synced_ls_files(&path_vec) {
                    Ok(ls_result) => {
                        match prepare_ls_output(ls_result) {
                            Ok(output) => {
                                trace!("**********************lsNative finished**************");
                                let array = js_sys::Uint8Array::from(output.as_slice());
                                Ok(array.into())
                            }
                            Err(msg) => {
                                trace!("wnfsError in ls_native output: {}", msg);
                                Err(JsValue::from_str(&msg))
                            }
                        }
                    }
                    Err(msg) => {
                        trace!("wnfsError in ls_native ls_res: {}", msg);
                        Err(JsValue::from_str(&msg.to_string()))
                    }
                }
            }
            Err(msg) => {
                trace!("wnfsError in ls_native forest_res: {}", msg);
                Err(JsValue::from_str(&msg.to_string()))
            }
        }
    })
}

    fn deserialize_cid(cid: String) -> Cid {
        Cid::try_from(cid).unwrap()
    }

    pub fn prepare_path_segments(path_segments: String) -> JsValue {
        let segments: Vec<String> = PrivateDirectoryHelper::parse_path(path_segments)
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        serde_wasm_bindgen::to_value(&segments).unwrap()
    }

    pub fn prepare_ls_output(ls_result: Vec<(String, Metadata)>) -> Result<Vec<u8>, String> {
        let mut result: Vec<u8> = Vec::new();
        let item_separator = "???".to_owned();
        let line_separator = "!!!".to_owned();
        
        for item in ls_result.iter() {
            let created = item.1.get_created();
            let modification = item.1.get_modified();
            
            if let (Some(created), Some(modification)) = (created, modification) {
                let filename = item.0.clone();
                let creation_time = created.to_string();
                let modification_time = modification.to_string();
                
                let row_string = format!("{}{}{}{}{}{}",
                    filename,
                    item_separator,
                    creation_time,
                    item_separator,
                    modification_time,
                    line_separator
                );
                
                let mut row_byte = row_string.into_bytes();
                result.append(&mut row_byte);
            }
        }
        
        Ok(result)
    }
}