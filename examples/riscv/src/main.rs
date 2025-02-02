#![no_std]
#![no_main]
// #![deny(warnings)]

#[macro_use]
extern crate log;

extern crate alloc;
extern crate opensbi_rt;

use alloc::vec;
use core::ptr::NonNull;
use fdt::{node::FdtNode, standard_nodes::Compatible, Fdt};
use log::LevelFilter;
use virtio_drivers::{
    device::{blk::VirtIOBlk, gpu::VirtIOGpu, input::VirtIOInput, net::VirtIONet},
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};
use virtio_impl::HalImpl;

mod virtio_impl;

#[cfg(feature = "tcp")]
mod tcp;

const NET_BUFFER_LEN: usize = 2048;
const NET_QUEUE_SIZE: usize = 16;

#[no_mangle]
extern "C" fn main(_hartid: usize, device_tree_paddr: usize) {
    log::set_max_level(LevelFilter::Debug);
    init_dt(device_tree_paddr);
    info!("test end");
}

fn init_dt(dtb: usize) {
    info!("device tree @ {:#x}", dtb);
    // Safe because the pointer is a valid pointer to unaliased memory.
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    walk_dt(fdt);
}

fn walk_dt(fdt: Fdt) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            if compatible.all().any(|s| s == "virtio,mmio") {
                virtio_probe(node);
            }
            if compatible.all().any(|s| s != "virtio,mmio") {
                debug!("Non virtio Device node {}", node.name);
            }
        }
    }
}

fn virtio_probe(node: FdtNode) {
    if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
        let paddr = reg.starting_address as usize;
        let size = reg.size.unwrap();
        let vaddr = paddr;
        info!("walk dt addr={:#x}, size={:#x}", paddr, size);
        info!(
            "Device tree node {}: {:?}",
            node.name,
            node.compatible().map(Compatible::first),
        );
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        let virtio_header_ref = unsafe { (vaddr as *mut VirtIOHeader).as_ref().unwrap() };
        info!("VirtIOHeader : {:?}",*virtio_header_ref);
        match unsafe { MmioTransport::new(header) } {
            Err(e) => warn!("Error creating VirtIO MMIO transport: {}", e),
            Ok(transport) => {
                info!(
                    "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                    transport.vendor_id(),
                    transport.device_type(),
                    transport.version(),
                );
                virtio_device(transport);
            }
        }
    }
}

fn virtio_device(transport: impl Transport) {
    match transport.device_type() {
        DeviceType::Block => virtio_blk(transport),
        DeviceType::GPU => virtio_gpu(transport),
        DeviceType::Input => virtio_input(transport),
        DeviceType::Network => virtio_net(transport),
        t => warn!("Unrecognized virtio device: {:?}", t),
    }
}

fn virtio_blk<T: Transport>(transport: T) {
    let mut blk = VirtIOBlk::<HalImpl, T>::new(transport).expect("failed to create blk driver");
    let mut input = vec![0xffu8; 512];
    let mut output = vec![0; 512];
    for i in 0..32 {
        for x in input.iter_mut() {
            *x = i as u8;
        }
        blk.write_block(i, &input).expect("failed to write");
        blk.read_block(i, &mut output).expect("failed to read");
        assert_eq!(input, output);
    }
    info!("virtio-blk test finished");
}

fn virtio_gpu<T: Transport>(transport: T) {
    let mut gpu = VirtIOGpu::<HalImpl, T>::new(transport).expect("failed to create gpu driver");
    let (width, height) = gpu.resolution().expect("failed to get resolution");
    let width = width as usize;
    let height = height as usize;
    info!("GPU resolution is {}x{}", width, height);
    // 设置显示缓冲区
    let fb = gpu.setup_framebuffer().expect("failed to get fb");
    let mut index = 0;
    let pic_info = [
        255, 247, 234, 240, 241, 236, 239, 253, 255, 252, 113, 48, 230, 88, 22, 218, 95, 2, 236,
        89, 12, 246, 85, 30, 228, 96, 13, 229, 87, 13, 233, 82, 1, 247, 93, 21, 230, 85, 32, 220,
        90, 15, 226, 95, 5, 232, 87, 22, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231,
        89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231,
        89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 231, 89, 17, 227, 91, 13, 232, 81, 24, 237,
        95, 9, 225, 89, 15, 234, 89, 24, 240, 89, 0, 234, 87, 10, 229, 86, 18, 226, 90, 6, 238, 86,
        3, 226, 92, 19, 233, 92, 21, 225, 86, 17, 245, 252, 255, 239, 251, 239, 255, 240, 238, 241,
        243, 240, 245, 255, 255, 233, 244, 246, 248, 123, 69, 228, 82, 22, 227, 95, 12, 236, 87, 5,
        227, 85, 9, 226, 91, 25, 230, 89, 33, 231, 88, 18, 234, 93, 14, 230, 92, 20, 228, 96, 11,
        228, 95, 0, 231, 89, 13, 228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16,
        228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16,
        228, 92, 16, 228, 92, 16, 228, 92, 16, 228, 92, 16, 232, 95, 14, 231, 86, 19, 232, 92, 5,
        226, 90, 16, 226, 87, 20, 228, 89, 6, 233, 92, 21, 232, 94, 19, 238, 92, 33, 239, 85, 21,
        227, 90, 20, 227, 88, 7, 235, 95, 10, 242, 247, 243, 225, 244, 240, 251, 250, 255, 236,
        254, 255, 234, 245, 249, 252, 245, 227, 228, 123, 58, 240, 98, 24, 227, 89, 16, 232, 89,
        13, 221, 99, 26, 230, 90, 5, 227, 84, 14, 227, 90, 12, 226, 92, 3, 228, 91, 10, 225, 83, 7,
        227, 80, 10, 237, 91, 34, 236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10,
        236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10,
        236, 86, 10, 236, 86, 10, 236, 86, 10, 236, 86, 10, 229, 84, 1, 234, 91, 13, 232, 87, 4,
        231, 84, 14, 231, 94, 22, 229, 97, 22, 225, 86, 19, 227, 85, 0, 235, 85, 0, 239, 87, 12,
        234, 93, 24, 232, 88, 18, 231, 81, 5, 255, 247, 245, 238, 243, 237, 242, 243, 247, 251,
        113, 50, 248, 122, 71, 230, 122, 60, 241, 139, 54, 253, 153, 55, 243, 148, 58, 255, 154,
        59, 247, 145, 44, 255, 148, 53, 255, 146, 65, 255, 153, 69, 240, 151, 51, 247, 154, 51,
        255, 155, 59, 255, 150, 59, 246, 147, 54, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251,
        154, 59, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251, 154,
        59, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251, 154, 59, 251, 154, 59,
        255, 156, 59, 242, 146, 46, 255, 155, 60, 255, 154, 68, 241, 146, 54, 245, 154, 65, 255,
        153, 68, 255, 153, 46, 241, 151, 55, 247, 163, 73, 233, 148, 55, 255, 163, 63, 255, 152,
        43, 87, 70, 40, 60, 53, 37, 64, 54, 52, 228, 84, 21, 227, 76, 19, 243, 95, 25, 255, 152,
        58, 253, 170, 52, 253, 175, 67, 239, 163, 67, 248, 168, 71, 250, 168, 66, 251, 159, 60,
        254, 164, 68, 249, 173, 75, 244, 169, 67, 249, 160, 60, 249, 161, 61, 245, 173, 62, 255,
        164, 65, 255, 164, 65, 255, 164, 65, 255, 164, 65, 255, 164, 65, 255, 164, 65, 255, 164,
        65, 255, 164, 65, 255, 164, 65, 255, 164, 65, 255, 164, 65, 255, 164, 65, 255, 164, 65,
        255, 164, 65, 255, 164, 65, 255, 164, 65, 249, 166, 64, 242, 171, 63, 250, 166, 67, 246,
        154, 55, 243, 166, 62, 254, 172, 72, 251, 159, 58, 242, 168, 63, 255, 172, 68, 234, 160,
        65, 250, 170, 71, 243, 167, 73, 247, 157, 61, 33, 26, 16, 4, 3, 17, 0, 0, 16, 225, 101, 11,
        232, 95, 15, 230, 86, 16, 244, 142, 57, 247, 166, 59, 255, 163, 52, 244, 163, 74, 255, 160,
        70, 236, 171, 87, 255, 171, 64, 255, 153, 46, 254, 160, 70, 252, 163, 73, 255, 159, 69,
        255, 157, 69, 252, 170, 71, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254,
        165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 255, 162, 67,
        246, 164, 62, 255, 161, 65, 255, 163, 60, 255, 170, 71, 255, 153, 54, 255, 163, 62, 234,
        173, 108, 255, 164, 56, 244, 164, 65, 255, 159, 49, 255, 166, 66, 255, 167, 65, 47, 36, 16,
        1, 2, 0, 6, 17, 0, 235, 92, 16, 236, 87, 7, 238, 90, 18, 255, 153, 62, 239, 163, 69, 239,
        159, 72, 9, 18, 0, 0, 0, 0, 20, 0, 0, 194, 131, 51, 255, 169, 63, 255, 169, 76, 239, 164,
        63, 255, 172, 61, 252, 164, 56, 238, 167, 53, 255, 166, 66, 255, 166, 66, 255, 166, 66,
        255, 166, 66, 255, 166, 66, 255, 166, 66, 255, 166, 66, 255, 166, 66, 255, 166, 66, 255,
        166, 66, 255, 166, 66, 255, 166, 66, 255, 166, 66, 255, 166, 66, 255, 166, 66, 255, 166,
        66, 255, 166, 64, 241, 163, 62, 255, 171, 70, 251, 173, 62, 226, 156, 61, 255, 166, 65,
        250, 167, 65, 11, 0, 0, 9, 0, 10, 18, 1, 0, 210, 135, 67, 240, 161, 69, 249, 166, 62, 52,
        31, 12, 10, 0, 5, 7, 2, 0, 238, 87, 32, 223, 85, 10, 219, 96, 26, 247, 144, 43, 249, 170,
        75, 255, 158, 66, 0, 1, 0, 6, 0, 21, 0, 0, 15, 196, 147, 70, 255, 161, 45, 253, 160, 64,
        245, 166, 65, 255, 168, 61, 251, 154, 59, 255, 171, 81, 255, 163, 64, 255, 163, 64, 255,
        163, 64, 255, 163, 64, 255, 163, 64, 255, 163, 64, 255, 163, 64, 255, 163, 64, 255, 163,
        64, 255, 163, 64, 255, 163, 64, 255, 163, 64, 255, 163, 64, 255, 163, 64, 255, 163, 64,
        255, 163, 64, 255, 159, 58, 255, 168, 71, 249, 157, 58, 250, 174, 62, 241, 169, 84, 255,
        153, 59, 251, 171, 76, 0, 6, 23, 0, 8, 24, 4, 4, 0, 209, 130, 55, 255, 166, 73, 249, 162,
        59, 49, 30, 15, 5, 1, 15, 0, 8, 0, 223, 89, 4, 228, 91, 23, 233, 91, 5, 255, 146, 51, 252,
        170, 68, 235, 170, 88, 17, 0, 0, 1, 4, 23, 6, 4, 0, 198, 130, 69, 255, 167, 61, 248, 167,
        52, 250, 160, 64, 255, 164, 71, 252, 165, 59, 247, 170, 56, 254, 159, 67, 246, 157, 73,
        255, 158, 59, 253, 151, 50, 255, 168, 61, 248, 161, 55, 255, 170, 64, 242, 155, 50, 255,
        162, 55, 248, 160, 54, 252, 173, 81, 255, 159, 54, 246, 159, 44, 255, 162, 68, 248, 163,
        56, 255, 160, 66, 255, 165, 72, 252, 165, 60, 236, 166, 68, 255, 171, 69, 252, 152, 58,
        255, 163, 67, 238, 170, 87, 0, 3, 0, 0, 3, 9, 26, 3, 0, 194, 141, 73, 251, 169, 69, 254,
        168, 47, 49, 31, 7, 3, 2, 0, 0, 0, 5, 236, 93, 17, 230, 87, 27, 232, 88, 17, 253, 142, 60,
        252, 160, 61, 255, 168, 63, 191, 128, 49, 187, 137, 64, 198, 137, 74, 251, 163, 76, 255,
        165, 60, 251, 168, 64, 255, 166, 72, 254, 166, 68, 255, 158, 64, 240, 110, 34, 238, 114,
        14, 234, 116, 28, 218, 99, 15, 232, 116, 41, 223, 107, 34, 227, 104, 34, 242, 109, 40, 239,
        98, 27, 244, 116, 53, 231, 108, 28, 234, 95, 30, 233, 99, 28, 234, 117, 40, 239, 105, 34,
        245, 170, 55, 243, 169, 60, 244, 162, 63, 255, 159, 57, 255, 163, 62, 255, 160, 53, 243,
        164, 69, 255, 174, 80, 255, 156, 47, 214, 128, 45, 191, 144, 64, 189, 120, 42, 245, 163,
        64, 255, 159, 62, 249, 157, 56, 46, 28, 24, 0, 0, 8, 1, 1, 9, 240, 88, 5, 230, 86, 15, 231,
        91, 12, 254, 152, 68, 251, 161, 65, 255, 156, 51, 255, 173, 69, 255, 166, 53, 255, 162, 56,
        250, 165, 58, 247, 174, 71, 247, 168, 75, 245, 161, 63, 235, 167, 56, 255, 166, 69, 239,
        91, 27, 221, 80, 9, 236, 93, 17, 230, 93, 23, 231, 95, 21, 225, 87, 12, 230, 93, 15, 236,
        102, 29, 222, 91, 23, 216, 88, 17, 223, 99, 0, 243, 90, 10, 234, 92, 18, 228, 99, 33, 230,
        71, 15, 254, 169, 62, 247, 170, 66, 253, 172, 57, 254, 167, 72, 250, 166, 80, 251, 169, 61,
        239, 169, 55, 244, 167, 63, 254, 157, 52, 246, 173, 81, 255, 167, 54, 255, 163, 62, 255,
        159, 55, 255, 161, 67, 251, 164, 67, 46, 33, 25, 0, 1, 0, 2, 3, 0, 241, 85, 11, 234, 91,
        12, 228, 94, 5, 247, 155, 56, 241, 165, 69, 255, 162, 72, 247, 159, 69, 250, 160, 64, 253,
        165, 55, 250, 167, 65, 241, 165, 71, 250, 159, 70, 255, 165, 67, 255, 175, 60, 253, 162,
        57, 227, 87, 10, 239, 91, 31, 248, 85, 6, 227, 89, 26, 227, 83, 10, 242, 93, 13, 244, 85,
        1, 233, 84, 4, 234, 92, 20, 237, 90, 21, 228, 92, 4, 227, 86, 7, 235, 87, 1, 231, 92, 9,
        243, 88, 22, 255, 158, 66, 252, 150, 65, 255, 162, 61, 255, 157, 58, 255, 155, 55, 255,
        163, 52, 244, 168, 58, 241, 165, 67, 255, 172, 79, 239, 163, 67, 239, 152, 57, 255, 165,
        78, 247, 159, 61, 253, 165, 67, 249, 168, 61, 46, 34, 20, 0, 1, 0, 6, 5, 0, 231, 84, 30,
        234, 93, 22, 231, 94, 13, 249, 153, 51, 243, 165, 64, 255, 168, 78, 239, 164, 63, 254, 175,
        74, 249, 160, 66, 255, 163, 75, 255, 161, 70, 255, 157, 60, 246, 118, 27, 226, 101, 11,
        228, 102, 17, 240, 93, 15, 227, 94, 25, 223, 87, 3, 85, 31, 0, 44, 13, 0, 36, 16, 5, 50,
        19, 0, 34, 11, 0, 36, 20, 7, 51, 18, 11, 51, 14, 8, 55, 22, 5, 184, 75, 16, 217, 89, 0,
        227, 95, 21, 245, 113, 31, 238, 98, 19, 236, 109, 38, 249, 150, 57, 255, 164, 64, 255, 165,
        78, 248, 168, 83, 254, 166, 77, 255, 154, 62, 255, 158, 50, 243, 174, 71, 252, 173, 80,
        248, 169, 66, 251, 164, 61, 252, 165, 59, 50, 32, 22, 1, 0, 7, 0, 0, 13, 225, 91, 18, 225,
        91, 6, 234, 91, 15, 255, 150, 55, 254, 164, 67, 255, 155, 67, 254, 168, 57, 249, 158, 51,
        255, 172, 76, 255, 159, 66, 249, 160, 56, 255, 170, 59, 226, 109, 16, 228, 86, 20, 232, 88,
        25, 239, 91, 21, 224, 84, 31, 240, 94, 17, 48, 15, 8, 5, 0, 6, 0, 4, 22, 8, 0, 0, 6, 4, 7,
        0, 2, 8, 0, 9, 0, 0, 0, 9, 0, 1, 0, 193, 82, 27, 238, 95, 3, 219, 89, 13, 232, 92, 17, 231,
        90, 11, 230, 87, 11, 252, 163, 61, 247, 162, 53, 247, 162, 71, 248, 164, 66, 255, 168, 62,
        253, 166, 71, 246, 168, 57, 246, 169, 53, 250, 162, 62, 255, 169, 66, 255, 160, 66, 255,
        162, 61, 53, 29, 17, 1, 0, 0, 0, 2, 2, 233, 99, 10, 224, 90, 0, 233, 86, 16, 252, 143, 52,
        252, 164, 64, 255, 156, 68, 255, 173, 64, 254, 157, 62, 244, 170, 61, 251, 160, 67, 244,
        166, 66, 251, 174, 66, 223, 112, 22, 233, 95, 32, 230, 92, 29, 225, 85, 8, 227, 94, 19,
        235, 88, 0, 38, 20, 0, 5, 11, 7, 0, 5, 14, 11, 0, 0, 10, 1, 0, 1, 1, 0, 4, 2, 5, 7, 0, 8,
        22, 7, 10, 181, 68, 28, 232, 88, 17, 234, 90, 29, 233, 91, 15, 229, 90, 5, 233, 88, 5, 236,
        151, 60, 254, 171, 75, 247, 164, 72, 250, 170, 55, 246, 155, 41, 251, 166, 75, 253, 171,
        46, 255, 163, 54, 255, 160, 69, 253, 163, 67, 255, 160, 68, 253, 160, 56, 48, 25, 7, 3, 3,
        0, 3, 9, 0, 234, 86, 22, 231, 88, 12, 242, 93, 37, 248, 147, 55, 242, 170, 59, 252, 170,
        71, 236, 165, 51, 255, 170, 80, 252, 166, 57, 255, 163, 78, 252, 161, 82, 255, 157, 69,
        240, 105, 24, 236, 89, 19, 225, 88, 10, 233, 96, 2, 236, 86, 35, 248, 88, 4, 34, 16, 16, 0,
        2, 0, 11, 8, 0, 202, 139, 72, 244, 161, 65, 255, 165, 51, 255, 167, 62, 248, 165, 61, 241,
        164, 72, 247, 119, 22, 232, 87, 0, 244, 84, 26, 226, 88, 16, 228, 94, 21, 233, 89, 19, 50,
        11, 0, 3, 2, 20, 9, 0, 0, 186, 134, 50, 255, 163, 62, 251, 154, 86, 248, 169, 74, 253, 161,
        58, 255, 168, 76, 235, 160, 59, 247, 169, 69, 250, 172, 64, 47, 29, 17, 3, 0, 5, 2, 0, 15,
        232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166,
        64, 254, 165, 65, 245, 162, 58, 250, 172, 64, 252, 162, 65, 254, 163, 80, 242, 113, 32,
        243, 95, 9, 218, 91, 12, 244, 82, 20, 240, 87, 9, 243, 83, 0, 46, 19, 8, 0, 1, 8, 2, 3, 0,
        197, 137, 64, 255, 163, 68, 247, 166, 61, 255, 163, 66, 251, 165, 66, 255, 167, 73, 230,
        105, 23, 230, 84, 9, 230, 87, 11, 226, 90, 12, 230, 95, 14, 232, 90, 18, 44, 15, 0, 0, 0,
        2, 0, 0, 4, 210, 132, 47, 250, 168, 68, 252, 164, 67, 255, 166, 66, 253, 166, 63, 255, 162,
        66, 255, 167, 70, 254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18,
        230, 90, 15, 234, 91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165,
        65, 255, 170, 68, 251, 165, 62, 253, 163, 66, 254, 166, 76, 224, 103, 20, 231, 92, 9, 218,
        102, 17, 234, 92, 18, 217, 94, 34, 225, 91, 20, 38, 19, 12, 2, 0, 6, 6, 0, 0, 202, 133, 58,
        255, 162, 71, 253, 169, 71, 253, 161, 62, 243, 161, 59, 253, 163, 67, 228, 106, 23, 235,
        89, 14, 237, 94, 18, 232, 95, 17, 230, 93, 15, 232, 90, 18, 44, 15, 0, 0, 0, 2, 0, 0, 4,
        210, 132, 47, 250, 168, 68, 252, 164, 67, 255, 166, 66, 253, 166, 63, 255, 162, 66, 255,
        167, 70, 254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90,
        15, 234, 91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 255,
        165, 64, 245, 157, 59, 250, 160, 64, 255, 176, 76, 236, 114, 29, 232, 88, 17, 219, 90, 7,
        231, 90, 11, 230, 87, 27, 235, 88, 21, 44, 10, 1, 9, 0, 4, 3, 0, 0, 201, 135, 59, 252, 163,
        69, 243, 170, 67, 255, 169, 66, 247, 168, 65, 255, 165, 68, 228, 107, 24, 230, 88, 14, 232,
        90, 16, 228, 88, 13, 227, 83, 10, 232, 90, 18, 44, 15, 0, 0, 0, 2, 0, 0, 4, 210, 132, 47,
        250, 168, 68, 252, 164, 67, 255, 166, 66, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254,
        166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13,
        252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 245, 159, 58, 255,
        174, 77, 255, 164, 71, 249, 154, 46, 232, 103, 20, 244, 84, 26, 241, 88, 12, 239, 90, 22,
        240, 95, 4, 232, 91, 9, 46, 10, 0, 8, 2, 4, 0, 2, 7, 207, 135, 63, 255, 157, 63, 255, 159,
        52, 250, 163, 60, 248, 166, 64, 255, 162, 66, 231, 109, 26, 226, 86, 11, 230, 89, 17, 233,
        91, 19, 239, 91, 21, 232, 90, 18, 44, 15, 0, 0, 0, 2, 0, 0, 4, 210, 132, 47, 250, 168, 68,
        252, 164, 67, 255, 166, 66, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60, 254,
        164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56,
        248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 246, 164, 62, 255, 167, 69, 254,
        167, 72, 255, 169, 58, 236, 118, 31, 223, 79, 16, 235, 95, 16, 207, 96, 41, 230, 99, 29,
        213, 90, 30, 53, 16, 0, 11, 2, 3, 0, 3, 7, 196, 135, 54, 251, 165, 64, 249, 176, 61, 255,
        165, 68, 255, 166, 70, 255, 159, 67, 241, 114, 33, 232, 92, 17, 229, 91, 18, 230, 89, 17,
        237, 86, 15, 232, 90, 18, 44, 15, 0, 0, 0, 2, 0, 0, 4, 210, 132, 47, 250, 168, 68, 252,
        164, 67, 255, 166, 66, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60, 254, 164,
        68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56, 248,
        165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254, 168, 67, 253, 161, 60, 245, 162,
        68, 249, 165, 53, 243, 153, 57, 255, 160, 78, 244, 151, 58, 58, 15, 0, 42, 12, 0, 60, 22,
        0, 0, 2, 0, 8, 7, 3, 2, 8, 8, 212, 134, 51, 255, 159, 65, 245, 166, 65, 228, 110, 22, 223,
        112, 23, 228, 109, 25, 227, 89, 14, 228, 86, 12, 229, 93, 17, 228, 90, 15, 236, 85, 14,
        232, 90, 18, 44, 15, 0, 0, 0, 2, 0, 0, 4, 210, 132, 47, 250, 168, 68, 252, 164, 67, 255,
        166, 66, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60, 254, 164, 68, 49, 30, 13,
        2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56, 248, 165, 63, 255,
        163, 65, 249, 166, 64, 254, 165, 65, 253, 165, 65, 255, 160, 55, 255, 167, 77, 255, 169,
        63, 243, 160, 64, 250, 167, 71, 243, 175, 76, 4, 1, 0, 0, 0, 10, 12, 1, 9, 0, 2, 4, 3, 0,
        0, 0, 2, 0, 208, 131, 41, 250, 164, 65, 239, 176, 73, 232, 92, 13, 224, 93, 13, 228, 92,
        16, 234, 88, 15, 235, 89, 16, 229, 96, 19, 228, 92, 14, 235, 87, 13, 232, 90, 18, 44, 15,
        0, 0, 0, 2, 0, 0, 4, 210, 132, 47, 250, 168, 68, 252, 164, 67, 255, 166, 66, 253, 166, 63,
        255, 162, 66, 255, 167, 70, 254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232,
        90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64,
        254, 165, 65, 252, 164, 66, 255, 162, 53, 255, 161, 73, 255, 161, 62, 255, 164, 71, 251,
        159, 60, 247, 171, 77, 2, 0, 20, 0, 3, 4, 3, 0, 0, 0, 5, 2, 5, 1, 0, 0, 4, 7, 219, 134, 44,
        255, 160, 64, 247, 165, 63, 238, 86, 13, 231, 87, 14, 234, 88, 15, 240, 87, 17, 234, 88,
        15, 225, 92, 13, 224, 91, 12, 238, 92, 15, 232, 90, 18, 44, 15, 0, 0, 0, 2, 0, 0, 4, 210,
        132, 47, 250, 168, 68, 252, 164, 67, 255, 166, 66, 253, 166, 63, 255, 162, 66, 255, 167,
        70, 254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15,
        234, 91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254,
        165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 254, 165, 65, 246, 167, 66, 251, 163, 63, 254, 166, 66, 248, 165, 63, 255, 160, 65,
        234, 108, 23, 235, 89, 12, 234, 88, 13, 241, 92, 10, 228, 88, 11, 218, 95, 25, 46, 11, 7,
        3, 4, 0, 4, 0, 4, 7, 7, 0, 12, 0, 4, 0, 0, 13, 0, 0, 11, 0, 6, 8, 2, 7, 13, 202, 124, 39,
        254, 168, 81, 252, 170, 68, 251, 163, 57, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254,
        166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13,
        252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254, 165, 65, 254,
        165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 252, 168, 69, 255, 165, 66, 254, 165, 65, 245, 163, 61, 253, 160, 64, 228, 106, 21,
        227, 87, 10, 230, 88, 14, 217, 85, 10, 229, 97, 14, 219, 87, 12, 52, 20, 9, 8, 7, 2, 0, 1,
        16, 4, 0, 0, 5, 0, 18, 7, 0, 0, 16, 5, 0, 13, 11, 0, 6, 0, 0, 217, 136, 45, 249, 159, 73,
        248, 162, 63, 255, 168, 68, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60, 254,
        164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56,
        248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254,
        165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 250, 161,
        61, 255, 162, 64, 255, 166, 66, 250, 171, 70, 255, 170, 74, 234, 115, 31, 227, 89, 14, 232,
        91, 19, 235, 95, 44, 231, 83, 9, 248, 86, 11, 50, 11, 0, 3, 0, 0, 0, 8, 19, 4, 8, 0, 0, 11,
        16, 0, 5, 15, 0, 0, 7, 0, 4, 0, 3, 0, 0, 211, 145, 61, 250, 170, 81, 245, 168, 64, 253,
        167, 58, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60, 254, 164, 68, 49, 30, 13,
        2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56, 248, 165, 63, 255,
        163, 65, 249, 166, 64, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 255, 167, 67, 255, 166, 68,
        252, 163, 63, 245, 163, 63, 253, 160, 65, 230, 109, 26, 227, 85, 13, 233, 89, 19, 232, 85,
        18, 230, 90, 5, 234, 89, 24, 31, 21, 19, 0, 0, 16, 2, 3, 0, 217, 132, 52, 217, 130, 50,
        208, 132, 44, 213, 135, 52, 206, 144, 61, 197, 132, 64, 255, 159, 52, 255, 159, 65, 248,
        160, 63, 255, 164, 74, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60, 254, 164,
        68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252, 149, 56, 248,
        165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 248, 162, 61,
        255, 164, 66, 255, 163, 64, 253, 169, 70, 255, 163, 69, 241, 116, 34, 233, 89, 16, 233, 89,
        18, 239, 91, 17, 231, 92, 1, 233, 87, 14, 42, 15, 0, 9, 0, 5, 3, 0, 0, 255, 167, 75, 246,
        157, 65, 255, 160, 67, 255, 155, 65, 247, 164, 72, 252, 167, 86, 249, 161, 53, 255, 166,
        65, 242, 167, 65, 241, 163, 62, 253, 166, 63, 255, 162, 66, 255, 167, 70, 254, 166, 60,
        254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234, 91, 13, 252,
        149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65,
        246, 164, 64, 255, 165, 66, 255, 164, 65, 255, 169, 68, 254, 157, 62, 238, 120, 33, 227,
        96, 16, 217, 90, 9, 243, 88, 8, 222, 90, 8, 223, 99, 35, 54, 19, 0, 3, 0, 0, 3, 5, 0, 238,
        151, 46, 255, 175, 68, 249, 168, 63, 253, 167, 66, 248, 170, 69, 255, 165, 69, 254, 167,
        62, 255, 160, 62, 255, 163, 64, 255, 163, 66, 253, 166, 63, 255, 162, 66, 255, 167, 70,
        254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15, 234,
        91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254, 165, 65,
        254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254,
        165, 65, 250, 168, 68, 255, 163, 67, 253, 159, 61, 252, 166, 63, 252, 163, 63, 255, 156,
        61, 255, 160, 68, 255, 163, 68, 0, 0, 4, 3, 13, 14, 15, 6, 1, 0, 0, 2, 0, 4, 7, 3, 1, 12,
        247, 163, 73, 243, 161, 77, 255, 163, 64, 255, 164, 67, 252, 166, 63, 254, 155, 54, 249,
        171, 70, 251, 162, 62, 246, 167, 64, 250, 167, 65, 253, 166, 63, 255, 162, 66, 255, 167,
        70, 254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 232, 90, 18, 230, 90, 15,
        234, 91, 13, 252, 149, 56, 248, 165, 63, 255, 163, 65, 249, 166, 64, 254, 165, 65, 254,
        165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165, 65, 254, 165,
        65, 254, 165, 65, 250, 166, 67, 255, 162, 67, 255, 162, 65, 255, 171, 68, 249, 166, 62,
        253, 164, 62, 253, 170, 68, 238, 167, 61, 19, 1, 0, 0, 0, 5, 0, 0, 0, 15, 4, 2, 4, 7, 0, 7,
        0, 0, 255, 173, 67, 253, 162, 57, 248, 165, 63, 253, 164, 64, 249, 168, 63, 255, 161, 57,
        242, 168, 71, 254, 161, 66, 253, 169, 70, 253, 163, 66, 253, 166, 63, 255, 162, 66, 255,
        167, 70, 254, 166, 60, 254, 164, 68, 49, 30, 13, 2, 0, 3, 0, 2, 0, 231, 94, 16, 229, 90, 9,
        235, 89, 6, 254, 147, 51, 249, 166, 64, 255, 163, 68, 255, 166, 64, 255, 161, 60, 254, 161,
        58, 252, 164, 64, 246, 167, 66, 245, 166, 65, 255, 170, 69, 255, 165, 66, 253, 154, 60,
        255, 160, 69, 251, 165, 66, 255, 168, 72, 255, 164, 65, 247, 168, 65, 246, 164, 62, 253,
        159, 61, 255, 163, 68, 255, 166, 70, 5, 0, 3, 0, 14, 1, 0, 2, 17, 0, 0, 14, 22, 2, 14, 3,
        4, 0, 243, 163, 66, 255, 158, 67, 253, 160, 57, 255, 172, 66, 247, 169, 60, 249, 168, 61,
        252, 163, 63, 255, 161, 67, 255, 164, 70, 254, 160, 62, 255, 170, 62, 254, 156, 65, 255,
        162, 69, 255, 165, 72, 248, 163, 57, 49, 36, 19, 0, 1, 0, 0, 4, 7, 233, 80, 23, 236, 89,
        22, 236, 93, 15, 252, 156, 56, 238, 167, 59, 251, 169, 67, 234, 159, 57, 253, 165, 68, 255,
        161, 65, 255, 164, 68, 251, 163, 65, 248, 159, 65, 245, 156, 66, 250, 161, 67, 254, 171,
        67, 244, 166, 55, 244, 162, 62, 252, 160, 61, 255, 161, 63, 255, 169, 69, 255, 160, 63,
        240, 120, 33, 234, 102, 20, 231, 102, 21, 186, 73, 5, 178, 76, 1, 184, 78, 20, 218, 129,
        37, 206, 135, 55, 192, 133, 53, 252, 171, 66, 237, 167, 69, 255, 174, 67, 246, 159, 66,
        254, 164, 76, 254, 163, 70, 255, 164, 65, 252, 163, 63, 251, 166, 60, 251, 171, 58, 242,
        163, 58, 254, 169, 78, 248, 168, 69, 249, 170, 69, 249, 173, 63, 41, 29, 15, 3, 0, 7, 10,
        0, 20, 241, 97, 11, 233, 91, 5, 229, 83, 0, 253, 149, 54, 250, 164, 65, 254, 160, 64, 255,
        177, 76, 254, 162, 63, 249, 166, 64, 255, 171, 65, 254, 166, 58, 255, 166, 66, 255, 165,
        72, 249, 159, 71, 249, 169, 74, 246, 171, 70, 253, 171, 71, 255, 164, 65, 255, 163, 64,
        253, 165, 65, 255, 159, 65, 237, 106, 24, 233, 85, 13, 238, 90, 18, 236, 85, 12, 221, 90,
        20, 225, 96, 30, 248, 147, 29, 240, 165, 63, 255, 170, 67, 254, 166, 66, 247, 164, 62, 244,
        166, 57, 255, 165, 77, 255, 162, 81, 251, 153, 56, 255, 170, 63, 255, 171, 72, 245, 167,
        67, 241, 164, 56, 254, 167, 61, 255, 162, 70, 253, 160, 64, 250, 155, 61, 251, 163, 57, 47,
        30, 10, 8, 6, 0, 10, 7, 0, 226, 88, 15, 230, 92, 19, 233, 86, 16, 255, 150, 64, 255, 165,
        66, 255, 158, 55, 249, 173, 61, 240, 164, 52, 246, 168, 68, 251, 169, 69, 243, 156, 51,
        244, 156, 46, 255, 171, 61, 255, 167, 63, 249, 156, 61, 255, 161, 75, 251, 165, 64, 255,
        164, 65, 253, 165, 65, 243, 164, 63, 254, 164, 68, 229, 110, 26, 226, 88, 15, 226, 88, 15,
        240, 91, 25, 224, 97, 26, 233, 81, 0, 255, 157, 48, 251, 164, 71, 243, 158, 65, 255, 164,
        69, 255, 162, 73, 245, 164, 57, 255, 167, 74, 255, 160, 61, 255, 166, 47, 253, 169, 53,
        242, 159, 67, 255, 169, 86, 255, 170, 77, 248, 176, 55, 245, 158, 53, 255, 170, 66, 255,
        163, 68, 254, 156, 57, 57, 34, 26, 1, 1, 3, 0, 1, 4, 234, 86, 22, 234, 92, 26, 225, 87, 15,
        246, 148, 57, 242, 165, 61, 255, 174, 75, 230, 160, 65, 243, 173, 87, 251, 162, 62, 255,
        165, 77, 255, 169, 85, 252, 165, 68, 250, 170, 55, 253, 173, 50, 250, 159, 45, 255, 158,
        58, 249, 163, 62, 255, 164, 65, 253, 165, 65, 243, 164, 63, 255, 166, 70, 233, 114, 30,
        229, 91, 18, 230, 92, 19, 237, 83, 13, 227, 98, 30, 239, 87, 20, 72, 41, 21, 56, 38, 28,
        54, 33, 6, 252, 164, 77, 255, 159, 64, 255, 168, 68, 253, 161, 60, 254, 164, 54, 255, 172,
        56, 243, 162, 57, 248, 162, 77, 255, 159, 72, 249, 148, 42, 240, 170, 75, 246, 166, 77,
        246, 169, 65, 252, 169, 67, 242, 159, 55, 46, 31, 24, 0, 0, 5, 1, 0, 10, 244, 90, 2, 231,
        90, 8, 224, 92, 17, 245, 154, 65, 249, 167, 67, 255, 154, 55, 255, 168, 67, 255, 154, 60,
        255, 169, 49, 249, 152, 49, 255, 157, 69, 255, 163, 78, 241, 162, 67, 245, 171, 66, 254,
        173, 68, 255, 170, 70, 254, 172, 72, 255, 164, 65, 254, 162, 63, 253, 165, 65, 255, 161,
        67, 238, 107, 25, 233, 85, 13, 236, 88, 16, 224, 93, 15, 225, 88, 7, 247, 81, 3, 31, 21, 9,
        0, 2, 0, 0, 5, 0, 243, 165, 64, 236, 174, 63, 255, 163, 68, 251, 162, 60, 253, 170, 68,
        245, 162, 70, 250, 164, 77, 255, 170, 78, 247, 159, 51, 255, 176, 55, 255, 151, 50, 255,
        163, 70, 244, 151, 48, 248, 161, 64, 252, 172, 73, 44, 31, 22, 1, 1, 0, 9, 3, 0, 241, 89,
        14, 231, 88, 18, 233, 91, 25, 250, 146, 61, 255, 164, 63, 255, 157, 56, 248, 165, 63, 243,
        163, 68, 229, 168, 75, 254, 179, 78, 253, 164, 60, 255, 161, 63, 254, 164, 76, 244, 162,
        78, 250, 169, 80, 244, 157, 60, 247, 165, 65, 253, 161, 62, 255, 161, 63, 255, 168, 68,
        255, 158, 61, 237, 117, 30, 227, 95, 13, 221, 92, 11, 230, 87, 27, 229, 90, 23, 240, 93,
        49, 47, 16, 22, 15, 0, 11, 4, 0, 2, 255, 167, 76, 252, 158, 62, 255, 161, 64, 249, 164, 57,
        251, 172, 71, 248, 162, 77, 255, 167, 77, 245, 157, 49, 242, 170, 68, 227, 171, 94, 254,
        172, 72, 251, 161, 65, 255, 172, 66, 255, 162, 64, 255, 165, 69, 42, 22, 21, 1, 0, 4, 6, 6,
        4, 228, 83, 16, 232, 92, 17, 234, 90, 4, 249, 144, 37, 251, 174, 70, 234, 173, 108, 11, 0,
        0, 0, 9, 26, 27, 0, 0, 187, 138, 79, 247, 170, 64, 255, 162, 42, 255, 164, 62, 247, 152,
        62, 255, 168, 68, 255, 167, 51, 252, 166, 67, 255, 167, 71, 255, 163, 64, 247, 168, 65,
        249, 167, 65, 255, 162, 64, 255, 164, 69, 253, 163, 67, 4, 3, 11, 0, 9, 0, 9, 1, 0, 0, 1,
        5, 6, 1, 5, 0, 11, 0, 242, 173, 72, 255, 159, 65, 254, 160, 60, 250, 170, 55, 252, 176, 66,
        249, 158, 65, 255, 157, 49, 254, 168, 47, 229, 173, 96, 30, 0, 0, 0, 1, 12, 23, 4, 0, 186,
        141, 73, 244, 166, 65, 255, 166, 49, 54, 32, 11, 1, 1, 1, 0, 3, 10, 229, 88, 6, 234, 91,
        25, 230, 90, 13, 239, 150, 68, 255, 161, 65, 255, 173, 59, 0, 0, 9, 3, 9, 9, 0, 4, 13, 195,
        146, 69, 255, 164, 51, 253, 164, 72, 240, 171, 68, 247, 167, 52, 255, 163, 55, 255, 165,
        61, 253, 166, 63, 253, 166, 63, 253, 166, 63, 253, 166, 63, 253, 166, 63, 253, 166, 63,
        253, 166, 63, 253, 166, 63, 0, 8, 10, 4, 0, 0, 0, 3, 2, 9, 0, 0, 1, 5, 14, 4, 0, 18, 255,
        178, 73, 253, 156, 61, 253, 167, 58, 243, 164, 59, 252, 165, 59, 251, 177, 56, 240, 167,
        73, 255, 152, 51, 255, 177, 76, 0, 2, 12, 16, 13, 4, 1, 0, 7, 210, 144, 60, 254, 164, 67,
        255, 153, 68, 45, 33, 9, 5, 6, 0, 0, 2, 0, 252, 96, 9, 245, 88, 9, 234, 85, 5, 241, 151,
        65, 251, 162, 68, 227, 157, 59, 12, 18, 18, 0, 2, 0, 24, 0, 0, 199, 127, 51, 255, 156, 58,
        255, 162, 76, 245, 164, 72, 253, 164, 64, 255, 159, 66, 251, 160, 67, 255, 163, 67, 255,
        163, 67, 255, 163, 67, 255, 163, 67, 255, 163, 67, 255, 163, 67, 255, 163, 67, 255, 163,
        67, 2, 0, 10, 12, 0, 6, 0, 5, 3, 9, 0, 0, 2, 5, 0, 16, 13, 6, 226, 164, 51, 241, 179, 70,
        254, 159, 67, 255, 174, 83, 255, 166, 75, 250, 161, 57, 246, 165, 76, 255, 160, 67, 250,
        160, 64, 25, 7, 0, 1, 0, 7, 8, 9, 11, 192, 138, 50, 243, 170, 55, 255, 169, 61, 31, 26, 4,
        0, 0, 13, 3, 3, 29, 217, 85, 21, 225, 84, 12, 231, 87, 16, 248, 153, 61, 255, 161, 53, 255,
        166, 49, 189, 134, 67, 207, 136, 58, 188, 135, 67, 254, 170, 74, 255, 160, 56, 254, 166,
        68, 251, 172, 67, 255, 171, 69, 252, 162, 66, 243, 168, 67, 252, 164, 67, 252, 164, 67,
        252, 164, 67, 252, 164, 67, 252, 164, 67, 252, 164, 67, 252, 164, 67, 252, 164, 67, 204,
        138, 44, 210, 134, 56, 191, 139, 53, 205, 136, 59, 201, 137, 76, 194, 133, 76, 255, 166,
        57, 248, 162, 61, 249, 158, 65, 244, 166, 66, 253, 160, 64, 255, 166, 62, 250, 171, 68,
        249, 156, 53, 255, 170, 64, 180, 135, 67, 209, 142, 61, 196, 142, 56, 254, 161, 58, 255,
        156, 55, 255, 164, 72, 40, 25, 18, 1, 4, 9, 0, 0, 5, 226, 96, 34, 233, 92, 12, 235, 91, 20,
        248, 149, 58, 251, 160, 53, 255, 160, 57, 240, 168, 70, 255, 165, 62, 252, 170, 70, 255,
        160, 63, 250, 151, 57, 251, 163, 65, 250, 163, 60, 255, 161, 67, 253, 158, 66, 251, 173,
        73, 255, 167, 61, 255, 167, 61, 255, 167, 61, 255, 167, 61, 255, 167, 61, 255, 167, 61,
        255, 167, 61, 255, 167, 61, 255, 166, 52, 255, 163, 72, 242, 168, 63, 252, 164, 66, 250,
        158, 73, 246, 163, 71, 255, 160, 55, 243, 165, 64, 255, 167, 74, 251, 172, 71, 255, 163,
        69, 255, 159, 64, 252, 166, 65, 251, 163, 66, 255, 162, 64, 245, 167, 66, 250, 160, 64,
        241, 168, 55, 255, 160, 62, 255, 159, 60, 253, 166, 69, 52, 29, 15, 9, 1, 0, 6, 0, 0, 232,
        87, 6, 241, 86, 0, 240, 90, 5, 251, 144, 50, 254, 166, 60, 255, 166, 78, 249, 167, 67, 255,
        162, 59, 253, 167, 46, 251, 162, 60, 255, 172, 75, 255, 175, 68, 249, 162, 56, 255, 166,
        64, 254, 161, 57, 244, 166, 58, 254, 164, 68, 254, 164, 68, 254, 164, 68, 254, 164, 68,
        254, 164, 68, 254, 164, 68, 254, 164, 68, 254, 164, 68, 254, 157, 62, 255, 158, 81, 254,
        169, 78, 255, 169, 73, 255, 157, 66, 255, 162, 49, 255, 163, 57, 247, 168, 63, 248, 163,
        57, 242, 166, 56, 251, 163, 57, 253, 157, 57, 247, 164, 60, 255, 178, 79, 255, 165, 69,
        255, 164, 47, 255, 156, 74, 250, 161, 57, 255, 162, 72, 255, 169, 72, 226, 160, 63, 54, 36,
        12, 3, 0, 0, 5, 0, 11, 34, 18, 19, 45, 15, 15, 42, 15, 8, 35, 30, 24, 29, 34, 12, 32, 28,
        19, 33, 26, 0, 47, 33, 6, 48, 32, 7, 33, 18, 11, 35, 25, 16, 38, 29, 14, 43, 24, 17, 57,
        33, 21, 52, 31, 12, 46, 29, 19, 44, 25, 8, 44, 25, 8, 44, 25, 8, 44, 25, 8, 44, 25, 8, 44,
        25, 8, 44, 25, 8, 44, 25, 8, 42, 36, 4, 42, 26, 3, 32, 25, 7, 36, 29, 3, 47, 23, 13, 45,
        29, 4, 45, 37, 26, 37, 28, 19, 51, 38, 21, 41, 27, 14, 46, 29, 9, 52, 32, 23, 38, 23, 16,
        35, 25, 15, 39, 19, 18, 58, 33, 13, 47, 35, 13, 35, 30, 8, 39, 24, 17, 48, 25, 9, 55, 37,
        13, 3, 4, 0, 0, 2, 0, 5, 2, 0, 5, 5, 5, 8, 0, 11, 5, 0, 0, 0, 1, 8, 0, 9, 0, 1, 0, 5, 7, 2,
        0, 0, 2, 0, 8, 9, 4, 0, 0, 5, 0, 5, 0, 6, 9, 2, 4, 3, 11, 0, 0, 0, 0, 2, 0, 7, 1, 11, 2, 0,
        3, 2, 0, 3, 2, 0, 3, 2, 0, 3, 2, 0, 3, 2, 0, 3, 2, 0, 3, 2, 0, 3, 0, 2, 1, 8, 1, 0, 6, 3,
        14, 2, 9, 1, 7, 0, 10, 2, 1, 0, 0, 5, 4, 6, 0, 0, 0, 0, 0, 5, 0, 9, 3, 3, 0, 4, 5, 7, 1, 2,
        7, 7, 7, 0, 4, 2, 7, 1, 0, 0, 0, 1, 0, 4, 6, 19, 4, 8, 11, 6, 0, 0, 4, 0, 0, 0, 3, 0, 0, 6,
        11, 0, 0, 5, 0, 6, 0, 1, 0, 14, 7, 9, 0, 0, 0, 10, 4, 7, 0, 6, 0, 0, 22, 7, 12, 0, 1, 5, 0,
        1, 6, 1, 2, 7, 3, 5, 0, 1, 0, 0, 1, 0, 14, 0, 7, 9, 0, 6, 0, 1, 0, 17, 0, 2, 0, 0, 2, 0, 0,
        2, 0, 0, 2, 0, 0, 2, 0, 0, 2, 0, 0, 2, 0, 0, 2, 0, 2, 1, 7, 13, 0, 0, 8, 0, 15, 0, 4, 0, 3,
        0, 13, 6, 5, 10, 0, 3, 2, 18, 0, 0, 0, 0, 4, 15, 3, 25, 2, 2, 0, 0, 5, 7, 0, 0, 9, 4, 1, 0,
        5, 5, 3, 0, 3, 11, 1, 3, 0, 3, 0, 27, 0, 1, 4, 12, 2, 0, 8, 3, 10, 3, 1, 0, 0, 0, 5, 6, 0,
        9,
    ];
    for y in 0..48 {
        for x in 0..48 {
            // 每个像素点 包含 BGR 三色的值
            // ex: 假定有2X2 4个点, 每个点用三个BGR值表示 , 那么数组就是 [255,255,255,100,100,100,200,200,200,50,50,50]
            //  |(255,255,255) | (100,100,100) |
            //  |--------------| --------------|
            //  |(200,200,200) | (50,50,50)    |
            let idx = (y * width + x) * 4;
            fb[idx] = pic_info[index + 2] as u8; // Blue
            fb[idx + 1] = pic_info[index + 1] as u8; // Green
            fb[idx + 2] = pic_info[index] as u8; //Red
            index = index + 3;
        }
    }
    gpu.flush().expect("failed to flush");
    //delay some time
    info!("virtio-gpu show graphics....");
    for _ in 0..100000 {
        for _ in 0..100000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }

    info!("virtio-gpu test finished");
}

fn virtio_input<T: Transport>(transport: T) {
    //let mut event_buf = [0u64; 32];
    let mut _input =
        VirtIOInput::<HalImpl, T>::new(transport).expect("failed to create input driver");
    // loop {
    //     input.ack_interrupt().expect("failed to ack");
    //     info!("mouse: {:?}", input.mouse_xy());
    // }
    // TODO: handle external interrupt
}

fn virtio_net<T: Transport>(transport: T) {
    let net = VirtIONet::<HalImpl, T, NET_QUEUE_SIZE>::new(transport, NET_BUFFER_LEN)
        .expect("failed to create net driver");
    info!("MAC address: {:02x?}", net.mac_address());

    #[cfg(not(feature = "tcp"))]
    {
        let mut net = net;
        loop {
            match net.receive() {
                Ok(buf) => {
                    info!("RECV {} bytes: {:02x?}", buf.packet_len(), buf.packet());
                    let tx_buf = virtio_drivers::device::net::TxBuffer::from(buf.packet());
                    net.send(tx_buf).expect("failed to send");
                    net.recycle_rx_buffer(buf).unwrap();
                    break;
                }
                Err(virtio_drivers::Error::NotReady) => continue,
                Err(err) => panic!("failed to recv: {:?}", err),
            }
        }
        info!("virtio-net test finished");
    }

    #[cfg(feature = "tcp")]
    tcp::test_echo_server(net);
}
