#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use core::net::Ipv4Addr;
use core::str::{FromStr, from_utf8};

use cyw43::JoinOptions;
use defmt::info;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::udp::UdpSocket;
use embassy_net::{Ipv4Cidr, Stack, StackResources, StaticConfigV4};
use embassy_rp::clocks::RoscRng;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use heapless::Vec;
use static_cell::StaticCell;

use crate::controller::Controller;
use crate::motor::Motors;
use crate::wifi::{Wifi, cyw43_task, net_task};

use {defmt_rtt as _, panic_probe as _};

mod controller;

mod display;
pub mod motor;
mod wifi;

embassy_rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<embassy_rp::peripherals::PIO0>;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

struct PioContext {}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let mut wifi = Wifi::init(p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29, p.DMA_CH0, p.PIO0).await;
    spawner.spawn(cyw43_task(wifi.runner)).unwrap();

    wifi.control.init(wifi::CLM).await;
    wifi.control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let mut dns_servers: Vec<Ipv4Addr, 3> = Vec::new();
    _ = dns_servers.push(Ipv4Addr::from_str(env!("WIFI_DNS_SERVER")).expect("valid dns server"));
    let config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(
            Ipv4Addr::from_str(env!("WIFI_ADDRESS")).expect("invalid wifi address"),
            24,
        ),
        gateway: Some(
            Ipv4Addr::from_str(env!("WIFI_GATEWAY_ADDRESS")).expect("invalid wifi gateway address"),
        ),
        dns_servers,
    });
    let seed = rng.next_u64();
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        wifi.driver,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(net_task(runner)).unwrap();

    let mac_address = wifi.control.address().await;
    info!(
        "Mac address: {:X}:{:X}:{:X}:{:X}:{:X}:{:X}",
        mac_address[0],
        mac_address[1],
        mac_address[2],
        mac_address[3],
        mac_address[4],
        mac_address[5],
    );

    info!("Waiting to connect to wifi: {}", env!("WIFI_SSID"));
    while let Err(err) = wifi
        .control
        .join(
            env!("WIFI_SSID"),
            JoinOptions::new(env!("WIFI_PASSWORD").as_bytes()),
        )
        .await
    {
        info!("failed to join network: {:?}", err.status);
    }
    info!("Connected to wifi network {}", env!("WIFI_SSID"));

    info!("Waiting for up wifi link up");
    stack.wait_link_up().await;
    info!("Waiting for up wifi config");
    stack.wait_config_up().await;
    info!("Done connecting to wifi");
    wifi.control.gpio_set(0, true).await;

    spawner
        .spawn(drive_motors_from_udp(
            Motors::init(
                (p.PWM_SLICE2, p.PIN_4, p.PIN_5),
                (p.PWM_SLICE3, p.PIN_6, p.PIN_7),
            ),
            stack,
        ))
        .expect("failed to spawn motor driver");
    // spawner
    //     .spawn(drive_motors_from_controller(
    //         Motors::init(
    //             (p.PWM_SLICE2, p.PIN_4, p.PIN_5),
    //             (p.PWM_SLICE3, p.PIN_6, p.PIN_7),
    //         ),
    //         Controller::init(p.PIN_9, p.PIN_11),
    //     ))
    //     .expect("failed to spawn motor driver");

    if let Ok(display) = display::Display::new((p.I2C0, p.PIN_1, p.PIN_0)).await {
        spawner
            .spawn(display::drive_display(display))
            .expect("failed to spawn display driver");
    }

    let delay = Duration::from_millis(10000);
    loop {
        // wifi_s.control.gpio_set(0, true).await;
        Timer::after(delay).await;
    }
    //
    // wifi_s.control.gpio_set(0, false).await;
    // Timer::after(delay).await;
    // }
}

#[embassy_executor::task]
async fn drive_motors_from_udp(mut motors: Motors, stack: Stack<'static>) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            defmt::warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    defmt::warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    defmt::warn!("read error: {:?}", e);
                    break;
                }
            };

            info!("rxd {}", from_utf8(&buf[..n]).unwrap());

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    defmt::warn!("write error: {:?}", e);
                    break;
                }
            };
        }
    }
}

#[embassy_executor::task]
async fn drive_motors_from_controller(mut motors: Motors, mut controller: Controller) {
    loop {
        let speed = controller.get_throttle().await;
        motors.drive_speed(speed, speed);
    }
}
