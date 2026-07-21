#![no_std]
#![no_main]

use core::net::Ipv4Addr;
use core::str::from_utf8;

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Stack, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use static_cell::StaticCell;

use crate::controller::Controller;
use crate::motor::{Motors, Speed};
use crate::wifi::{Wifi, cyw43_task, net_task};

use {defmt_rtt as _, panic_probe as _};

mod controller;
mod leasehund;
use leasehund::DhcpServer;

mod display;
pub mod motor;
mod wifi;

embassy_rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<embassy_rp::peripherals::PIO0>;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

struct PioContext {}

#[embassy_executor::task]
async fn dhcp_task(stack: embassy_net::Stack<'static>) {
    // Create the DHCP server configuration
    // The generic parameters <32, 4> mean: max 32 clients, max 4 DNS servers
    let mut dhcp_server: DhcpServer<32, 4> = DhcpServer::new_with_dns(
        Ipv4Addr::new(192, 168, 1, 1),   // Server IP (Your Pico's IP)
        Ipv4Addr::new(255, 255, 255, 0), // Subnet mask
        Ipv4Addr::new(192, 168, 1, 1),   // Gateway IP (Your Pico)
        Ipv4Addr::new(8, 8, 8, 8),       // DNS server (e.g., Google DNS)
        Ipv4Addr::new(192, 168, 1, 50),  // IP pool start (First IP to assign)
        Ipv4Addr::new(192, 168, 1, 200), // IP pool end (Last IP to assign)
    );

    // Run the DHCP server in an infinite loop
    dhcp_server.run(stack).await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let mut wifi = Wifi::init(p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29, p.DMA_CH0, p.PIO0).await;
    spawner.spawn(cyw43_task(wifi.runner)).unwrap();
    wifi.control.init(wifi::CLM).await;

    // Use a link-local address for communication without DHCP server
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 1, 1), 24),
        dns_servers: heapless::Vec::new(),
        gateway: None,
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
    spawner.spawn(dhcp_task(stack)).unwrap();
    wifi.control
        .set_power_management(cyw43::PowerManagementMode::Performance)
        .await;

    wifi.control
        .start_ap_wpa2("PICO CAR AAAAA", "password", 5)
        .await;

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
        motors.drive_speed(Speed::OFF, Speed::OFF);
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(5)));

        info!("Listening on TCP:8133...");
        if let Err(e) = socket.accept(8133).await {
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
            match buf.first().map(|x| *x as char) {
                Some('w') => {
                    motors.drive_speed(Speed::from_percent(1.), Speed::from_percent(1.));
                }
                Some('s') => {
                    motors.drive_speed(Speed::from_percent(-1.), Speed::from_percent(-1.));
                }
                Some('a') => {
                    motors.drive_speed(Speed::from_percent(0.), Speed::from_percent(1.));
                }
                Some('d') => {
                    motors.drive_speed(Speed::from_percent(1.), Speed::from_percent(0.));
                }
                _ => {
                    motors.drive_speed(Speed::from_percent(0.), Speed::from_percent(0.));
                }
            }

            info!("rxd {}", from_utf8(&buf[..n]).unwrap());

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    motors.drive_speed(Speed::OFF, Speed::OFF);
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
