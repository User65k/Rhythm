use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

mod ws;
mod ctx;
mod list;

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
}

//#[wasm_bindgen(start)]
#[wasm_bindgen]
pub fn init_ui() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    if let Err(e) = 
        list::setup(&document)
        .and_then(|_| ctx::setup_ctx_men(&document) )
        .and_then(|_| {
            let loc = document.location().unwrap();
            ws::start_websocket(loc)
        }) {
        log(e);
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