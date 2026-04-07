#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use tokio::runtime::Runtime;

use po_node::Po;

pub struct PoClientC {
    inner: tokio::sync::Mutex<Po>,
    rt: Runtime,
}

#[no_mangle]
pub extern "C" fn po_client_new(
    bind_address_or_port: *const c_char,
    remote_address: *const c_char,
) -> *mut PoClientC {
    if bind_address_or_port.is_null() {
        return ptr::null_mut();
    }

    let bind_str = unsafe {
        CStr::from_ptr(bind_address_or_port)
            .to_string_lossy()
            .into_owned()
    };

    let remote_str = if remote_address.is_null() {
        None
    } else {
        Some(unsafe {
            CStr::from_ptr(remote_address)
                .to_string_lossy()
                .into_owned()
        })
    };

    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(_) => return ptr::null_mut(),
    };

    let inner = rt.block_on(async {
        if let Some(ref remote) = remote_str {
            Po::connect(remote).await
        } else {
            let port: u16 = bind_str.parse().unwrap_or(0);
            Po::bind(port).await
        }
    });

    match inner {
        Ok(po) => {
            let client = Box::new(PoClientC {
                inner: tokio::sync::Mutex::new(po),
                rt,
            });
            Box::into_raw(client)
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn po_client_send(client: *mut PoClientC, data: *const u8, len: usize) -> i32 {
    if client.is_null() || data.is_null() {
        return -1;
    }

    let c = unsafe { &*client };
    let payload = unsafe { std::slice::from_raw_parts(data, len) };

    let res =
        c.rt.block_on(async { c.inner.lock().await.send(payload).await });

    match res {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn po_client_free(client: *mut PoClientC) {
    if !client.is_null() {
        unsafe {
            let _ = Box::from_raw(client);
        }
    }
}
