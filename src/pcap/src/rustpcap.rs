use libc::{c_char,c_int};
use std::ptr;
use std::ffi::CString;

mod pcap {
    #![allow(dead_code)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    bindgen!("/usr/include/pcap.h", link="pcap");
}

//TODO: http://www.tcpdump.org/linktypes.html
pub type DataLinkType = c_int;
pub const DLT_NULL: DataLinkType = 0;
pub const DLT_ETHERNET: DataLinkType = 1;
pub const DLT_IEEE802_11_RADIO: DataLinkType = 127;

#[derive(Copy)]
pub struct PcapSessionBuilder {
    p: *mut pcap::pcap_t,
    activated: bool
}

pub fn list_devices() {
    println!("list_devices")
}

impl PcapSessionBuilder {

    pub fn new_dev(dev: &str) -> Result<PcapSessionBuilder, &'static str> {
        let mut errbuf = Vec::with_capacity(256);
        let c_dev = CString::new(dev.as_bytes()).unwrap().as_ptr();
        PcapSessionBuilder::do_new(c_dev, errbuf.as_mut_slice())
    }

    pub fn new() -> Result<PcapSessionBuilder, &'static str> {
        let mut errbuf = Vec::with_capacity(256);
        let dev = unsafe { pcap::pcap_lookupdev(errbuf.as_mut_slice().as_mut_ptr()) };
        if dev.is_null() {
            Err("No device available")
        } else {
            PcapSessionBuilder::do_new(dev as *const c_char, errbuf.as_mut_slice())
        }
    }

    fn do_new(dev: *const c_char, errbuf: &mut [c_char]) -> Result<PcapSessionBuilder, &'static str> {
        let p = unsafe { pcap::pcap_create(dev, errbuf.as_mut_ptr()) };
        if p.is_null() {
            Err("Could not initialize device")
        } else {
            Ok(PcapSessionBuilder { p: p, activated: false })
        }
    }

    pub fn buffer_size(&mut self, sz: i32) -> &mut PcapSessionBuilder {
        if self.activated { panic!("Session already activated") }
        unsafe { pcap::pcap_set_buffer_size(self.p, sz); }
        self
    }

    pub fn timeout(&mut self, to: i32) -> &mut PcapSessionBuilder {
        if self.activated { panic!("Session already activated") }
        unsafe { pcap::pcap_set_timeout(self.p, to); }
        self
    }

    pub fn promisc(&mut self, promisc: bool) -> &mut PcapSessionBuilder {
        if self.activated { panic!("Session already activated") }
        unsafe { pcap::pcap_set_promisc(self.p, promisc as c_int); }
        self
    }

    pub fn rfmon(&mut self, rfmon: bool) -> &mut PcapSessionBuilder {
        if self.activated { panic!("Session already activated") }
        unsafe { pcap::pcap_set_rfmon(self.p, rfmon as c_int); }
        self
    }

    pub fn activate(&mut self) -> PcapSession {
        if self.activated { panic!("Session already activated") }
        unsafe { pcap::pcap_activate(self.p); }
        self.activated = true;
        PcapSession { p: self.p }
    }
}

#[derive(Copy)]
pub struct PcapSession {
    p: *mut pcap::pcap_t
}

impl PcapSession {
    pub fn from_file(f: &str) -> PcapSession {
        let mut errbuf = Vec::with_capacity(256);
        unsafe {
            let p = pcap::pcap_open_offline(CString::new(f.as_bytes()).unwrap().as_ptr(),
                                            errbuf.as_mut_slice().as_mut_ptr());
            PcapSession { p: p }
        }
    }

    pub fn datalink(&self) -> DataLinkType {
        unsafe { pcap::pcap_datalink(self.p) }
    }

    pub fn list_datalinks(&self) -> Vec<i32> {
        unsafe {
            let mut dlt_buf = ptr::null_mut();
            let sz = pcap::pcap_list_datalinks(self.p, &mut dlt_buf);
            let out = Vec::from_raw_buf(dlt_buf as *const c_int, sz as usize);
            pcap::pcap_free_datalinks(dlt_buf);
            out
        }
    }

    //TODO: add a return value for success/failure
    pub fn next<F>(&self, mut f: F) where F: FnMut(&PcapData) {
        let mut head_ptr = ptr::null_mut();
        let mut data_ptr = ptr::null();
        let res = unsafe { pcap::pcap_next_ex(self.p, &mut head_ptr, &mut data_ptr) };
        match res {
            0 => return, //timed out
            1 => {
                let p = PcapData { hdr: head_ptr, dat: data_ptr };
                f(&p);
            }
            _ => {
                panic!("pcap_next_ex panicked with {}, find something better to do than blow up",
                       res);
            }
        }
    }
}


pub struct PcapTimeval(pcap::Struct_timeval);

impl PcapTimeval {
    pub fn sec(&self) -> i64 {
        self.0.tv_sec
    }

    pub fn usec(&self) -> i32 {
        self.0.tv_usec
    }
}

pub struct PcapData {
    hdr: *mut pcap::Struct_pcap_pkthdr,
    dat: *const u8
}

impl PcapData {
    pub fn len(&self) -> u32 {
        unsafe { (*self.hdr).len }
    }

    pub fn caplen(&self) -> u32 {
        unsafe { (*self.hdr).caplen }
    }

    pub fn ts(&self) -> PcapTimeval {
        unsafe { PcapTimeval((*self.hdr).ts) }
    }

    pub fn pkt_ptr(&self) -> *const u8 {
        self.dat
    }
}

pub struct PcapDumper {
    p: *mut pcap::Struct_pcap_dumper
}

impl PcapDumper {
    pub fn new(sess: &PcapSession, path: &str) -> PcapDumper {
        unsafe {
            let p = pcap::pcap_dump_open(sess.p, CString::new(path.as_bytes()).unwrap().as_ptr());
            PcapDumper { p: p }
        }
    }

    pub fn dump(&mut self, data: &PcapData) {
        unsafe {
            pcap::pcap_dump(self.p as *mut u8, data.hdr, data.dat);
        }
    }
}

impl Drop for PcapDumper {
    fn drop(&mut self) {
        unsafe {
            pcap::pcap_dump_close(self.p);
        }
    }
}
