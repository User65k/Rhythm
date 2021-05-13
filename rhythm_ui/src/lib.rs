use wasm_bindgen::prelude::*;
use web_sys::{Document, window};

mod ws;
mod ctx;
mod list;
mod resize;

type WebRes = Result<(),JsValue>;
fn unwrap_some<T>(v: Option<T>) -> Result<T,JsValue> {
    if let Some(val) = v {
        Ok(val)
    }else{
        Err(JsValue::from_str("expected value"))
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: JsValue);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn error(s: JsValue);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn error2(s: JsValue, s2: JsValue);
}

static mut DOCUMENT: Option<Document> = None;

fn get_document_ref() -> &'static Document {
    unsafe {
      DOCUMENT.as_ref().expect("No document root")
    }
}
fn show_error(err: &str) {
  //TODO show Toast
  //and log to the console
  error(JsValue::from_str(err));
}
fn show_error_w_val(err: &str, js_err: JsValue) {
  //TODO show Toast
  //and log to the console
  error2(JsValue::from_str(err), js_err);
}

//#[wasm_bindgen(start)]
#[wasm_bindgen]
pub fn init_ui() {
    let window = window().expect("no global `window` exists");
    let document = window.document();
    unsafe {
      DOCUMENT = document;
    }
    let document = get_document_ref();

    if let Err(e) = 
        list::setup(document)
        .and_then(|_| ctx::setup_ctx_men(document) )
        .and_then(|_| resize::setup_resize(document) )
        .and_then(|_| {
            let loc = document.location().unwrap();
            ws::start_websocket(loc)
        }) {
          show_error_w_val("could not initialize", e);
    }
}


/*
fetch('coffee.jpg')
.then(response => {
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  } else {
    return response.blob();
  }
})
.then(myBlob => {
  let objectURL = URL.createObjectURL(myBlob);
  let image = document.createElement('img');
  image.src = objectURL;
  document.body.appendChild(image);
})
.catch(e => {
  console.log('There has been a problem with your fetch operation: ' + e.message);
});
*/