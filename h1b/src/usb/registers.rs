// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::ops::{BitAnd, BitOr};
use kernel::common::cells::VolatileCell;
use kernel::common::registers::{self, ReadOnly, ReadWrite, WriteOnly};

register_bitfields![u32,
    AhbConfig [  // OTG Databook, Table 5-9
        GlobalInterruptMask                OFFSET(0)  NUMBITS(1) [],
        BurstLength                        OFFSET(1)  NUMBITS(4) [
            Len1Word    = 0b0000,
            Len4Words   = 0b0001,
            Len8Words   = 0b0010,
            Len16Words  = 0b0011,
            Len32Words  = 0b0100,
            Len64Words  = 0b0101,
            Len128Words = 0b0110,
            Len256Words = 0b0111
        ],
        DmaEnable                          OFFSET(5)  NUMBITS(1) [],
        NonPeriodicTxFifoEmptyLevel        OFFSET(7)  NUMBITS(1) [],
        PeriodicTxFifoEmptyLevel           OFFSET(8)  NUMBITS(1) [],
        RemoteMemorySupport                OFFSET(21) NUMBITS(1) [],
        NotifyAllDmaWrite                  OFFSET(22) NUMBITS(1) [],
        AhbSingleSupport                   OFFSET(23) NUMBITS(1) [],
        InverseDescEndianness              OFFSET(24) NUMBITS(1) []
    ],

    UsbConfiguration [  // OTG Databook, Table 5-10
        TimeoutCalibration                 OFFSET(0)  NUMBITS(3) [],
        PhysicalInterface                  OFFSET(3)  NUMBITS(1) [
            Bits8  = 0,
            Bits16 = 1
        ],
        UlpiUtmiSelect                     OFFSET(4)  NUMBITS(1) [
            Utmi = 0,
            Ulpi = 1
        ],
        FullSpeedSerialInterfaceSelect     OFFSET(5)  NUMBITS(1) [
            Unidirectional6Pin = 0,
            Bidirectional3Pin  = 1
        ],
        PhySelect                          OFFSET(6)  NUMBITS(1) [
            Usb20HighSpeed     = 0,
            Usb11FullSpeed     = 1
        ],
        UlpiDdrSelect                      OFFSET(7)  NUMBITS(1) [
            SingleDataRate8bit = 0,
            DoubleDataRate4bit = 1
        ],
        SrpCapable                         OFFSET(8)  NUMBITS(1) [],
        HnpCapable                         OFFSET(9)  NUMBITS(1) [],
        UsbTurnaroundTime                  OFFSET(10) NUMBITS(4) []
        // Bit 14 reserved
        // Bits 15+ not used by SW; not included because they won't be tested
    ],

    Reset [  // OTG Databook, Table 5-11
        AhbMasterIdle                      OFFSET(31) NUMBITS(1) [],
        DmaRequestSignal                   OFFSET(30) NUMBITS(1) [],
        TxFifoNumber                       OFFSET(6)  NUMBITS(5) [
            Fifo0       =  0,
            Fifo1       =  1,
            Fifo2       =  2,
            Fifo3       =  3,
            Fifo4       =  4,
            Fifo5       =  5,
            Fifo6       =  6,
            Fifo7       =  7,
            Fifo8       =  8,
            Fifo9       =  9,
            Fifo10      = 10,
            Fifo11      = 11,
            Fifo12      = 12,
            Fifo13      = 13,
            Fifo14      = 14,
            Fifo15      = 15,
            AllFifos    = 16 // It's 5 bits, 0x10 means all FIFOs
        ],
        TxFifoFlush                        OFFSET(5)  NUMBITS(1) [],
        RxFifoFlush                        OFFSET(4)  NUMBITS(1) [],
        InTokenSequenceLearningQueueFlush  OFFSET(3)  NUMBITS(1) [],
        HostFrameCounterReset              OFFSET(2)  NUMBITS(1) [],
        PiuFsDedicatedControllerSoftReset  OFFSET(1)  NUMBITS(1) []
    ],

    Interrupt [  // OTG Databook, Table 5-13
        // Note this field is not valid on the Mask register
        CurrentMode                        OFFSET(0)  NUMBITS(1) [
            Host   = 0b0,
            Device = 0b1
        ],
        ModeMismatch                       OFFSET(1)  NUMBITS(1) [],
        OTG                                OFFSET(2)  NUMBITS(1) [],
        StartOfFrame                       OFFSET(3)  NUMBITS(1) [],
        RxFifoNotEmpty                     OFFSET(4)  NUMBITS(1) [],
        NonPeriodicTxFifoEmpty             OFFSET(5)  NUMBITS(1) [],
        GlobalInNak                        OFFSET(6)  NUMBITS(1) [],
        GlobalOutNak                       OFFSET(7)  NUMBITS(1) [],
        EarlySuspend                       OFFSET(10) NUMBITS(1) [],
        Suspend                            OFFSET(11) NUMBITS(1) [],
        Reset                              OFFSET(12) NUMBITS(1) [],
        EnumerationDone                    OFFSET(13) NUMBITS(1) [],
        OutIsochronousPacketDropped        OFFSET(14) NUMBITS(1) [],
        EndOfPeriodicFrame                 OFFSET(15) NUMBITS(1) [],
        RestoreDone                        OFFSET(16) NUMBITS(1) [],
        EndpointMismatch                   OFFSET(17) NUMBITS(1) [],
        InEndpoints                        OFFSET(18) NUMBITS(1) [],
        OutEndpoints                       OFFSET(19) NUMBITS(1) [],
        IncompleteIsochronousInTransfer    OFFSET(20) NUMBITS(1) [],
        IncompletePeriodicTransfer         OFFSET(21) NUMBITS(1) [],
        DataFetchSuspended                 OFFSET(22) NUMBITS(1) [],
        ResetDetected                      OFFSET(23) NUMBITS(1) [],
        ConnectIDChange                    OFFSET(28) NUMBITS(1) [],
        DisconnectDetected                 OFFSET(29) NUMBITS(1) [],
        SessionRequest                     OFFSET(30) NUMBITS(1) [],
        ResumeWakeup                       OFFSET(31) NUMBITS(1) []
    ],

    Gpio [  // OTG Databook, Table 5-22
        Gpi                                OFFSET(0)  NUMBITS(16) [],
        GpoRegister                        OFFSET(16) NUMBITS(4)  [],
        GpoValue                           OFFSET(20) NUMBITS(8)  [],
        GpoOperation                       OFFSET(31) NUMBITS(1)  [
            Read  = 0,
            Write = 1
        ]
    ],

    DeviceConfig [  // OTG Databook, Table 5-53
        DeviceSpeed                        OFFSET(0) NUMBITS(2) [
            High  = 0b00,
            Full2 = 0b01,
            Low   = 0b10,
            Full1 = 0b11
        ],
        DeviceAddress                      OFFSET(4)  NUMBITS(7) [],
        PeriodicFrameInterval              OFFSET(11) NUMBITS(2) [
            Interval80 = 0b00,
            Interval85 = 0b01,
            Interval90 = 0b10,
            Interval95 = 0b11
        ],
        EnableDeviceOutNak                 OFFSET(13) NUMBITS(1) [],
        XcvrDelay                          OFFSET(14) NUMBITS(1) [],
        ErraticErrorInterruptMask          OFFSET(15) NUMBITS(1) [],
        InEndpointMismatchCount            OFFSET(18) NUMBITS(5) [],
        EnableScatterGatherDMAInDeviceMode OFFSET(23) NUMBITS(1) [],
        PeriodicScheduling                 OFFSET(24) NUMBITS(2) [
            Interval25 = 0b00,
            Interval50 = 0b01,
            Interval75 = 0b10
        ],
        ResumeValidationPeriod             OFFSET(26) NUMBITS(6) []
    ],

    DeviceControl [  // OTG Databook, Table 5-54
        RemoteWakeupSignaling              OFFSET(0)  NUMBITS(1) [],
        SoftDisconnect                     OFFSET(1)  NUMBITS(1) [],
        GlobalNonPeriodicInNakStatus       OFFSET(2)  NUMBITS(1) [],
        GlobalOutNakStatus                 OFFSET(3)  NUMBITS(1) [],
        TestControl                        OFFSET(4)  NUMBITS(3) [
            Disabled        = 0b000,
            ModeJ           = 0b001,
            ModeK           = 0b010,
            ModeSE0Nak      = 0b011,
            ModePacket      = 0b100,
            ModeForceEnable = 0b101
        ],
        SetGlobalNonPeriodicInNak          OFFSET(7)  NUMBITS(1) [],
        ClearGlobalNonPeriodicInNak        OFFSET(8)  NUMBITS(1) [],
        SetGlobalOutNak                    OFFSET(9)  NUMBITS(1) [],
        ClearGlobalOutNak                  OFFSET(10) NUMBITS(1) [],
        PowerOnProgrammingDone             OFFSET(11) NUMBITS(1) [],
        GlobalMultiCount                   OFFSET(13) NUMBITS(2) [
            CountInvalid   = 0b00,
            Count1Packet   = 0b01,
            Count2Packets  = 0b10,
            Count3Packets  = 0b11
        ],
        IgnoreFrameNumber                  OFFSET(15) NUMBITS(1) [],
        NakOnBabbleError                   OFFSET(16) NUMBITS(1) [],
        EnableContinueOnBna                OFFSET(17) NUMBITS(1) [],
        DeepSleepBESLReject                OFFSET(18) NUMBITS(1) []
    ],

    InEndpointInterruptMask [  // OTG Databook, Table 5-57
        TransferCompleted                0,
        EndpointDisabled                 1,
        AhbError                         2,
        Timeout                          3,
        InTokenReceivedWhenTxFifoEmpty   4,
        InTokenEndpointMismatched        5,
        InEndpointNakEffective           6,
        // Bit 7 reserved
        TxFifoUnderrun                   8,
        BufferNotAvailable               9,
        // Bits 10-12 reserved
        NAK                             13
        // Bits 14-31 reserved
    ],

    OutEndpointInterruptMask [  // OTG Databook, Table 5-58
        TransferCompleted                     0,
        EndpointDisabled                      1,
        AhbError                              2,
        SetupPhaseDone                        3,
        OutTokenReceivedWhenEndpointDisabled  4,
        StatusPhaseReceived                   5,
        BackToBackSetupPacketsReceived        6,
        // Bit 7 reserved
        OutPacketError                        8,
        BnaInterrupt                          9,
        // Bits 10-11 reserved
        BabbleError                          12,
        Nak                                  13,
        Nyet                                 14
        // Bits 15-31 reserved
    ],

    AllEndpointInterrupt [  // OTG Databook Table 5-59
        IN0    0,
        IN1    1,
        IN2    2,
        IN3    3,
        IN4    4,
        IN5    5,
        IN6    6,
        IN7    7,
        IN8    8,
        IN9    9,
        IN10  10,
        IN11  11,
        IN12  12,
        IN13  13,
        IN14  14,
        IN15  15,
        OUT0  16,
        OUT1  17,
        OUT2  18,
        OUT3  19,
        OUT4  20,
        OUT5  21,
        OUT6  22,
        OUT7  23,
        OUT8  24,
        OUT9  25,
        OUT10 26,
        OUT11 27,
        OUT12 28,
        OUT13 29,
        OUT14 30,
        OUT15 31
    ],

    EndpointControl [
        MaximumPacketSize                  OFFSET(0)  NUMBITS(11) [],
        NextEndpoint                       OFFSET(11) NUMBITS(4)  [],
        UsbActiveEndpoint                  OFFSET(15) NUMBITS(1)  [],
        NakStatus                          OFFSET(17) NUMBITS(1)  [
            TransmittingNonNakHandshakes = 0,
            TransmittingNakHandshakes    = 1
        ],
        EndpointType                       OFFSET(18) NUMBITS(2)  [
            Control     = 0b00,
            Isochronous = 0b01,
            Bulk        = 0b10,
            Interrupt   = 0b11
        ],
        SnoopMode                          OFFSET(20) NUMBITS(1)  [],
        Stall                              OFFSET(21) NUMBITS(1)  [],
        TxFifoNumber                       OFFSET(22) NUMBITS(4)  [],
        ClearNak                           OFFSET(26) NUMBITS(1)  [],
        SetNak                             OFFSET(27) NUMBITS(1)  [],
        Disable                            OFFSET(30) NUMBITS(1)  [],
        Enable                             OFFSET(31) NUMBITS(1)  []
    ]
];


#[repr(C)]
pub struct Registers {
    pub _otg_control: VolatileCell<u32>,
    pub _otg_interrupt: VolatileCell<u32>,
    pub ahb_config: ReadWrite<u32, AhbConfig::Register>,
    pub configuration: ReadWrite<u32, UsbConfiguration::Register>,
    pub reset: ReadWrite<u32, Reset::Register>,
    pub interrupt_status: ReadWrite<u32, Interrupt::Register>,
    pub interrupt_mask: ReadWrite<u32, Interrupt::Register>,
    pub _grxstsr: VolatileCell<u32>,
    pub _grxstsp: VolatileCell<u32>,
    pub receive_fifo_size: VolatileCell<u32>,
    pub transmit_fifo_size: VolatileCell<u32>,

    _reserved: [u32; 3],
    // 0x38
    /// The `gpio` register is a portal to a set of custom 8-bit registers.
    ///
    /// Logically it is split into a GP_OUT part and a GP_IN part. Writing to a
    /// custom register can be done in a single operation, with all data
    /// transferred in GP_OUT. Reading requires a GP_OUT write to select the
    /// register to read, then a read or GP_IN to see what the register holds.
    ///   GP_OUT:
    ///    bit  15     direction: 1=write, 0=read
    ///    bits 11:4   value to write to register when bit 15 is set
    ///    bits 3:0    custom register to access
    ///   GP_IN:
    ///    bits 7:0    value read back from register when GP_OUT[15] is clear
    pub gpio: ReadWrite<u32, Gpio::Register>,
    pub _guid: VolatileCell<u32>,
    pub _gsnpsid: VolatileCell<u32>,
    pub _user_hw_config: [VolatileCell<u32>; 4],

    _reserved0: [u32; 2],

    pub _gdfifocfg: VolatileCell<u32>,

    _reserved1: [u32; 41],

    pub device_in_ep_tx_fifo_size: [VolatileCell<u32>; 15],

    _reserved2: [u32; 432],

    pub device_config: ReadWrite<u32, DeviceConfig::Register>,
    pub device_control: ReadWrite<u32, DeviceControl::Register>,
    pub _device_status: VolatileCell<u32>,

    _reserved_3: u32,
    // 0x810
    pub device_in_ep_interrupt_mask: ReadWrite<u32, InEndpointInterruptMask::Register>,  // DIEPMASK
    pub device_out_ep_interrupt_mask: ReadWrite<u32, OutEndpointInterruptMask::Register>, // DOEPMASK
    pub device_all_ep_interrupt: ReadWrite<u32, AllEndpointInterrupt::Register>,      // DAINT
    pub device_all_ep_interrupt_mask: ReadWrite<u32, AllEndpointInterrupt::Register>, // DAINTMASK

    _reserved_4: [u32; 2],
    // 0x828
    pub _device_vbus_discharge_time: VolatileCell<u32>,
    pub _device_vbus_pulsing_time: VolatileCell<u32>,
    pub _device_threshold_control: VolatileCell<u32>,
    pub _device_in_ep_fifo_empty_interrupt_mask: VolatileCell<u32>,

    _reserved_5: [u32; 50],
    // 0x900
    pub in_endpoints: [InEndpoint; 16],
    // 0xb00
    pub out_endpoints: [OutEndpoint; 16],
    // 0xd00
    _reserved6: [u32; 64],
    // 0xe00
    pub _power_clock_gating_control: VolatileCell<u32>,
}

#[repr(C)]
pub struct InEndpoint {
    pub control: ReadWrite<u32, EndpointControl::Register>,
    _reserved0: u32,
    pub interrupt: ReadWrite<u32, InEndpointInterruptMask::Register>,
    _reserved1: u32,
    // We use scatter-gather mode so transfer-size isn't used
    _transfer_size: VolatileCell<u32>,
    pub dma_address: VolatileCell<&'static DMADescriptor>,
    pub tx_fifo_status: VolatileCell<u32>,
    pub buffer_address: VolatileCell<u32>,
}

#[repr(C)]
pub struct OutEndpoint {
    pub control: ReadWrite<u32, EndpointControl::Register>,
    _reserved0: u32,
    pub interrupt: ReadWrite<u32, OutEndpointInterruptMask::Register>,
    _reserved1: u32,
    _transfer_size: VolatileCell<u32>,
    pub dma_address: VolatileCell<&'static DMADescriptor>,
    _reserved2: u32,
    pub buffer_address: VolatileCell<u32>,
}

/// In/Out Endpoint Control flags
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EpCtl(pub u32);

impl EpCtl {
    /// Enable the endpoint
    pub const ENABLE: EpCtl = EpCtl(1 << 31);
    /// Clear endpoint NAK
    pub const CNAK: EpCtl = EpCtl(1 << 26);
    /// Stall endpoint
    pub const STALL: EpCtl = EpCtl(1 << 21);
    /// Snoop on bad frames
    pub const SNOOP: EpCtl = EpCtl(1 << 20);
    /// Make an endpoint of type Interrupt
    pub const INTERRUPT: EpCtl = EpCtl(3 << 18);
    /// Denotes whether endpoint is active
    pub const USBACTEP: EpCtl = EpCtl(1 << 15);

    pub const TXFNUM_0: EpCtl = EpCtl(0 << 22);
    pub const TXFNUM_1: EpCtl = EpCtl(1 << 22);
    pub const TXFNUM_2: EpCtl = EpCtl(2 << 22);
    pub const TXFNUM_3: EpCtl = EpCtl(3 << 22);

    pub const TXFNUM_4: EpCtl = EpCtl(4 << 22);
    pub const TXFNUM_5: EpCtl = EpCtl(5 << 22);
    pub const TXFNUM_6: EpCtl = EpCtl(6 << 22);
    pub const TXFNUM_7: EpCtl = EpCtl(7 << 22);

    pub const TXFNUM_8: EpCtl = EpCtl(8 << 22);
    pub const TXFNUM_9: EpCtl = EpCtl(9 << 22);
    pub const TXFNUM_10: EpCtl = EpCtl(10 << 22);
    pub const TXFNUM_11: EpCtl = EpCtl(11 << 22);

    pub const TXFNUM_12: EpCtl = EpCtl(12 << 22);
    pub const TXFNUM_13: EpCtl = EpCtl(13 << 22);
    pub const TXFNUM_14: EpCtl = EpCtl(14 << 22);
    pub const TXFNUM_15: EpCtl = EpCtl(15 << 22);

    // EP0 has a different control register layout than the other
    // endpoints (EPN). In EP0, the MPS field is 2 bits; in EPN, it is
    // 10 bits (sections 5.3.5.21 and 5.3.5.22 in the OTG databook. A
    // better implementation would type check this. -pal
    pub const MPS_EP0_64: EpCtl = EpCtl(0 << 0);
    pub const MPS_EP0_32: EpCtl = EpCtl(1 << 0);
    pub const MPS_EP0_16: EpCtl = EpCtl(2 << 0);
    pub const MPS_EP0_8: EpCtl = EpCtl(3 << 0);

    pub fn epn_mps(self, cnt: u32) -> EpCtl {
        self | EpCtl(cnt & 0x3ff)
    }

    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

impl BitOr for EpCtl {
    type Output = Self;
    fn bitor(self, rhs: EpCtl) -> EpCtl {
        EpCtl(self.0 | rhs.0)
    }
}

impl BitAnd for EpCtl {
    type Output = Self;
    fn bitand(self, rhs: EpCtl) -> EpCtl {
        EpCtl(self.0 & rhs.0)
    }
}




#[repr(C)]
#[repr(align(4))]
#[derive(Clone, Copy, Debug)]
pub struct DMADescriptor {
    pub flags: DescFlag,
    pub addr: usize,
}

/// Status quadlet for a DMA descriptor
///
/// The status quadlet is a 32-bit flag register in the DMA descriptor that
/// reflects the status of the descriptor. It can mark whether the Host/DMA is
/// ready to transmit/receive this descriptor and describes how large the buffer
/// is.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescFlag(pub u32);

impl BitOr for DescFlag {
    type Output = Self;
    fn bitor(self, rhs: DescFlag) -> DescFlag {
        DescFlag(self.0 | rhs.0)
    }
}

impl BitAnd for DescFlag {
    type Output = Self;
    fn bitand(self, rhs: DescFlag) -> DescFlag {
        DescFlag(self.0 & rhs.0)
    }
}

impl DescFlag {
    /// This descriptor is the last in a transmission
    pub const LAST: DescFlag = DescFlag(1 << 27);
    /// This descriptor describes a short transfer
    pub const SHORT: DescFlag = DescFlag(1 << 26);
    /// Generate an interrupt on completion
    pub const IOC: DescFlag = DescFlag(1 << 25);
    /// Indicates that a setup packet has been received
    pub const SETUP_READY: DescFlag = DescFlag(1 << 24);

    // Mask for pulling out status bits
    pub const STATUS_MASK: DescFlag = DescFlag(0b11 << 30);
    /// Host Ready status
    pub const HOST_READY: DescFlag = DescFlag(0b00 << 30);
    /// DMA Busy status
    pub const DMA_BUSY: DescFlag = DescFlag(0b01 << 30);
    /// DMA Ready status
    pub const DMA_DONE: DescFlag = DescFlag(0b10 << 30);
    /// Host Busy status
    pub const HOST_BUSY: DescFlag = DescFlag(0b11 << 30);

    /// Set the number of bytes to transmit
    pub const fn bytes(self, bytes: u16) -> DescFlag {
        DescFlag(self.0 | bytes as u32)
    }

    pub const fn to_u32(self) -> u32 {
        self.0
    }
}
