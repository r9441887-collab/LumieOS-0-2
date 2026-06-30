use crate::drivers::rtl8168::*;
use core::ptr::copy_nonoverlapping;

const NET_RX_BUF_SIZE: u32 = 2048;

static mut G_NET_INITIALIZED: i32 = 0;
static mut G_NET_MAC: [u8; 6] = [0; 6];
static mut G_NET_RX_BUF: [u8; NET_RX_BUF_SIZE as usize] = [0; NET_RX_BUF_SIZE as usize];

pub unsafe fn net_init() -> i32 {
    if G_NET_INITIALIZED != 0 {
        return 0;
    }

    let ret = rtl8168_init();
    if ret < 0 {
        return -1;
    }

    let ret2 = rtl8168_get_mac(G_NET_MAC.as_mut_ptr());
    if ret2 < 0 {
        return -1;
    }

    G_NET_INITIALIZED = 1;
    0
}

pub unsafe fn net_send(data: *const u8, len: u32) -> i32 {
    if G_NET_INITIALIZED == 0 {
        return -1;
    }
    rtl8168_send_packet(data, len)
}

pub unsafe fn net_receive(buf: *mut u8, max_len: *mut u32) -> i32 {
    if G_NET_INITIALIZED == 0 {
        return -1;
    }

    let mut pkt_len: u32 = 0;
    let ret = rtl8168_receive_packet(G_NET_RX_BUF.as_mut_ptr(), &mut pkt_len as *mut u32);
    if ret < 0 {
        return -1;
    }

    if !buf.is_null() && pkt_len > 0 {
        if !max_len.is_null() && *max_len < pkt_len {
            pkt_len = *max_len;
        }
        copy_nonoverlapping(G_NET_RX_BUF.as_ptr(), buf, pkt_len as usize);
    }

    if !max_len.is_null() {
        *max_len = pkt_len;
    }
    0
}

pub unsafe fn net_get_mac(mac: *mut u8) -> i32 {
    if G_NET_INITIALIZED == 0 {
        return -1;
    }
    if mac.is_null() {
        return -1;
    }
    copy_nonoverlapping(G_NET_MAC.as_ptr(), mac, 6);
    0
}
