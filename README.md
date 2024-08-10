# Esp32-oled-0.96-inch on Rust demo

* display on oled
* Wifi connect
* Https client
* Crypto Price display (BTC/ETH/SOL)

![IMG_20240810_101439.jpeg](IMG_20240810_101439.jpeg)
![IMG_20240810_101508.jpg](IMG_20240810_101508.jpg)

# build and flash and monitor
cargo build --release && espflash flash -p /dev/ttyUSB0 target/xtensa-esp32-espidf/release/rust-esp32-oled-0-96-inch --monitor

```text
[2024-08-09T15:40:59Z INFO ] Serial port: '/dev/ttyUSB0'
[2024-08-09T15:40:59Z INFO ] Connecting...
[2024-08-09T15:41:00Z INFO ] Using flash stub
Chip type:         esp32 (revision v3.1)
Crystal frequency: 40 MHz
Flash size:        4MB
Features:          WiFi, BT, Dual Core, 240MHz, Coding Scheme None
MAC address:       08:a6:f7:22:99:d4
App/part. size:    397,696/4,128,768 bytes, 9.63%
[2024-08-09T15:41:01Z INFO ] Segment at address '0x1000' has not changed, skipping write
[2024-08-09T15:41:01Z INFO ] Segment at address '0x8000' has not changed, skipping write
[00:00:23] [========================================]     230/230     0x10000                                                                                                                                                                                                                                      [2024-08-09T15:41:26Z INFO ] Flashing has completed!
Commands:
    CTRL+R    Reset chip
    CTRL+C    Exit

ets Jul 29 2019 12:21:46

rst:0x1 (POWERON_RESET),boot:0x17 (SPI_FAST_FLASH_BOOT)
configsip: 0, SPIWP:0xee
clk_drv:0x00,q_drv:0x00,d_drv:0x00,cs0_drv:0x00,hd_drv:0x00,wp_drv:0x00
mode:DIO, clock div:2
load:0x3fff0030,len:7104
load:0x40078000,len:15576
load:0x40080400,len:4
0x40080400 - _invalid_pc_placeholder
    at ??:??
ho 8 tail 4 room 4
load:0x40080404,len:3876
entry 0x4008064c
I (31) boot: ESP-IDF v5.1-beta1-378-gea5e0ff298-dirt 2nd stage bootloader
I (31) boot: compile time Jun  7 2023 07:48:23
I (33) boot: Multicore bootloader
I (37) boot: chip revision: v3.1
I (41) boot.esp32: SPI Speed      : 40MHz
I (46) boot.esp32: SPI Mode       : DIO
I (50) boot.esp32: SPI Flash Size : 4MB
I (55) boot: Enabling RNG early entropy source...
I (60) boot: Partition Table:
I (64) boot: ## Label            Usage          Type ST Offset   Length
I (71) boot:  0 nvs              WiFi data        01 02 00009000 00006000
I (79) boot:  1 phy_init         RF data          01 01 0000f000 00001000
I (86) boot:  2 factory          factory app      00 00 00010000 003f0000
I (94) boot: End of partition table
I (98) esp_image: segment 0: paddr=00010020 vaddr=3f400020 size=12720h ( 75552) map
I (134) esp_image: segment 1: paddr=00022748 vaddr=3ffb0000 size=0229ch (  8860) load
I (137) esp_image: segment 2: paddr=000249ec vaddr=40080000 size=0b62ch ( 46636) load
I (159) esp_image: segment 3: paddr=00030020 vaddr=400d0020 size=3ff10h (261904) map
I (254) esp_image: segment 4: paddr=0006ff38 vaddr=4008b62c size=01218h (  4632) load
I (262) boot: Loaded app from partition at offset 0x10000
I (262) boot: Disabling RNG early entropy source...
I (276) cpu_start: Multicore app
I (285) cpu_start: Pro cpu start user code
I (285) cpu_start: cpu freq: 160000000 Hz
I (285) app_init: Application information:
I (288) app_init: Project name:     libespidf
I (293) app_init: App version:      9e90370-dirty
I (298) app_init: Compile time:     Aug  9 2024 23:02:29
I (304) app_init: ELF file SHA256:  000000000...
I (309) app_init: ESP-IDF:          v5.4-dev-1388-g5ca9f2a49a
I (316) efuse_init: Min chip rev:     v0.0
I (321) efuse_init: Max chip rev:     v3.99 
I (326) efuse_init: Chip rev:         v3.1
I (331) heap_init: Initializing. RAM available for dynamic allocation:
I (338) heap_init: At 3FFAE6E0 len 00001920 (6 KiB): DRAM
I (344) heap_init: At 3FFB2BF0 len 0002D410 (181 KiB): DRAM
I (350) heap_init: At 3FFE0440 len 00003AE0 (14 KiB): D/IRAM
I (356) heap_init: At 3FFE4350 len 0001BCB0 (111 KiB): D/IRAM
I (363) heap_init: At 4008C844 len 000137BC (77 KiB): IRAM
I (370) spi_flash: detected chip: generic
I (373) spi_flash: flash io: dio
W (378) pcnt(legacy): legacy driver is deprecated, please migrate to `driver/pulse_cnt.h`
W (386) i2c: This driver is an old driver, please migrate your application code to adapt `driver/i2c_master.h`
W (397) timer_group: legacy driver is deprecated, please migrate to `driver/gptimer.h`
I (406) main_task: Started on CPU0
I (416) main_task: Calling app_main()
I (416) rust_esp32_oled_0_96_inch: Hello, world!

```