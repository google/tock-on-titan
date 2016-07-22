use cortexm3;

pub struct Hotel {
    pub mpu: cortexm3::mpu::MPU
}

impl Hotel {
    pub unsafe fn new() -> Hotel {
        Hotel { mpu: cortexm3::mpu::MPU::new() }
    }
}
