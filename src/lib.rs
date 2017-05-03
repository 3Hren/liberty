extern crate curl;
extern crate futures;
extern crate libc;
extern crate tokio_core;
extern crate tokio_curl;

use std::io;
use std::slice;
use std::str;
use std::ptr;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};

use curl::easy::Easy;

use futures::{Future, Stream};
use futures::sync::mpsc::{self, UnboundedSender};

use tokio_core::reactor::Core;
use tokio_curl::Session;

enum Event {
    Perform(Request),
    Shutdown,
}

pub type Callback = extern fn(*const Error, *const Response, *mut libc::c_void);

#[derive(Debug)]
pub struct Error<'a> {
    desc: Option<&'a str>,
}

pub struct Request {
    handle: Easy,
    complete: Option<(Callback, *mut libc::c_void)>,
}

unsafe impl Send for Request {}

#[derive(Debug)]
pub struct Header {
    pub name: String,
    pub value: Vec<u8>,
}

pub struct Response {
    code: u32,
    status: String,
    headers: Vec<Header>,
    body: Vec<u8>,
}

impl Response {
    fn new() -> Self {
        Self {
            code: 0,
            status: String::new(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
}

pub struct HttpClient {
    tx: UnboundedSender<Event>,
    thread: Option<JoinHandle<Result<(), io::Error>>>,
}

impl HttpClient {
    fn new() -> Self {
        let (tx, rx) = mpsc::unbounded();

        let thread = thread::spawn(move || {
            let mut core = Core::new()?;
            let handle = core.handle();
            let session = Session::new(core.handle());

            let future = rx.for_each(|event| {
                match event {
                    Event::Perform(mut request) => {
                        match request.complete {
                            Some((callback, data)) => {
                                let response = Arc::new(Mutex::new(Response::new()));
                                {
                                    let response = response.clone();
                                    request.handle.header_function(move |header| {
                                        let mut response = response.lock().expect("lock must be healthy");

                                        if response.status.is_empty() {
                                            match String::from_utf8(header.into()) {
                                                Ok(status) => {
                                                    response.status = status;
                                                    true
                                                }
                                                Err(..) => false,
                                            }
                                        } else {
                                            let mut iter = header.splitn(2, |c| *c == b':');
                                            let name = iter.next().and_then(|name| str::from_utf8(name).ok());
                                            let value = iter.next();

                                            if let (Some(name), Some(value)) = (name, value) {
                                                let header = Header {
                                                    name: name.into(),
                                                    value: value.into()
                                                };

                                                response.headers.push(header);
                                            }
                                            true
                                        }
                                    }).unwrap();
                                }
                                {
                                    let response = response.clone();
                                    request.handle.write_function(move |data| {
                                        response.lock().expect("lock must be healthy").body.extend_from_slice(data);
                                        Ok(data.len())
                                    }).unwrap();
                                }

                                let future = session.perform(request.handle)
                                    .then(move |handle| {
                                        match handle {
                                            Ok(mut handle) => {
                                                let mut response = response.lock().expect("lock must be healthy");
                                                response.code = handle.response_code().unwrap_or(0);
                                                callback(ptr::null(), &*response, data);
                                            }
                                            Err(err) => {
                                                let err = err.into_error();
                                                let desc = err.to_string();
                                                let e = Error {
                                                    desc: Some(&desc),
                                                };
                                                callback(&e, ptr::null(), data);
                                            }
                                        }

                                        Ok(())
                                    });
                                handle.spawn(future);
                            }
                            None => {
                                let future = session.perform(request.handle)
                                    .then(|_handle| {
                                        Ok(())
                                    });
                                handle.spawn(future);
                            }
                        }
                        Ok(())
                    }
                    Event::Shutdown => {
                        // Halt the iteration.
                        println!("Shutdown!");
                        Err(())
                    }
                }
            });

            core.run(future).err().expect("the core future can be only stopped with error");

            println!("Halt!");

            Ok(())
        });

        Self {
            tx: tx,
            thread: Some(thread),
        }
    }
}

impl Drop for HttpClient {
    fn drop(&mut self) {
        self.tx.send(Event::Shutdown).expect("channel must live");
        self.thread.take().unwrap().join().unwrap().unwrap();
    }
}

#[no_mangle]
pub extern fn liberty_http_request_make() -> *mut Request {
    let request = Request {
        handle: Easy::new(),
        complete: None,
    };
    let request = Box::new(request);

    Box::into_raw(request)
}

#[no_mangle]
pub extern fn liberty_http_request_free(request: *mut Request) {
    if request != ptr::null_mut() {
        unsafe { Box::from_raw(request) };
    }
}

#[no_mangle]
pub extern fn liberty_http_request_get(request: *mut Request) -> libc::c_int {
    let mut request = unsafe { &mut *request };

    request.handle.get(true)
        .map_err(|err| err.code())
        .err()
        .unwrap_or(0) as libc::c_int
}

#[no_mangle]
pub extern fn liberty_http_request_post(request: *mut Request) -> libc::c_int {
    let mut request = unsafe { &mut *request };

    request.handle.post(true)
        .map_err(|err| err.code())
        .err()
        .unwrap_or(0) as libc::c_int
}

#[no_mangle]
pub extern fn liberty_http_request_url(request: *mut Request, data: *mut u8, size: libc::size_t) -> libc::c_int {
    let request = unsafe { &mut *request };
    let slice = unsafe { slice::from_raw_parts(data, size) };
    let url = str::from_utf8(slice).unwrap();

    request.handle.url(url)
        .map_err(|err| err.code())
        .err()
        .unwrap_or(0) as libc::c_int
}

#[no_mangle]
pub extern fn liberty_http_request_data(request: *mut Request, data: *mut u8, size: libc::size_t) -> libc::c_int {
    let request = unsafe { &mut *request };
    let data = unsafe { slice::from_raw_parts(data, size) };

    request.handle.post_fields_copy(data)
        .map_err(|err| err.code())
        .err()
        .unwrap_or(0) as libc::c_int
}

#[no_mangle]
pub extern fn liberty_http_request_complete_callback(request: *mut Request, callback: Callback, data: *mut libc::c_void) {
    let request = unsafe { &mut *request };

    request.complete = Some((callback, data));
}

#[no_mangle]
pub extern fn liberty_http_client_make() -> *mut HttpClient {
    let client = Box::new(HttpClient::new());
    Box::into_raw(client)
}

#[no_mangle]
pub extern fn liberty_http_client_free(client: *mut HttpClient) {
    let client = unsafe { Box::from_raw(client) };
    drop(client)
}

#[no_mangle]
pub extern fn liberty_http_client_perform(client: *mut HttpClient, request: *mut Request) {
    let client = unsafe { &*client };
    let request = unsafe { Box::from_raw(request) };

    client.tx.send(Event::Perform(*request)).expect("channel must be opened")
}

#[no_mangle]
pub extern fn liberty_http_response_code(response: *const Response) -> libc::c_int {
    let response = unsafe { &*response };

    response.code as libc::c_int
}

#[no_mangle]
pub extern fn liberty_http_response_body(response: *const Response) -> *const u8 {
    let response = unsafe { &*response };

    response.body.as_ptr()
}

#[no_mangle]
pub extern fn liberty_http_response_body_size(response: *const Response) -> libc::size_t {
    let response = unsafe { &*response };

    response.body.len()
}

#[no_mangle]
pub extern fn liberty_error_extra(error: *const Error) -> *const u8 {
    assert!(error != ptr::null());
    let error = unsafe { &*error };

    error.desc.unwrap_or("").as_ptr()
}

#[no_mangle]
pub extern fn liberty_error_extra_size(error: *const Error) -> libc::size_t {
    assert!(error != ptr::null());
    let error = unsafe { &*error };

    error.desc.unwrap_or("").len()
}
