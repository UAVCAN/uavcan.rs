use core::mem::transmute;

trait UavcanSerializable {
    fn uavcan_size(&self) -> usize;
}

pub struct Bool {
    value: bool,
}

pub struct IntX {
    x: usize,
    value: i64,
}

pub struct UintX {
    x: usize,
    value: u64,
}

pub struct Float16 {
    positive: bool,
    exponent: u8,
    fraction: u16,
}

pub struct Float32 {
    value: f32,
}

pub struct Float64 {
    value: f64,
}

pub struct VoidX{
    x: usize,
}

}

impl From<Bool> for bool {
    fn from(t: Bool) -> bool {
        t.value
    }
}

impl From<IntX> for i64 {
    fn from(t: IntX) -> i64 {
        t.value
    }
}

impl From<UintX> for u64 {
    fn from(t: UintX) -> u64 {
        t.value
    }
}

impl From<Float16> for f32 {
    fn from(t: Float16) -> f32 {
        let positive_f32 = t.positive;
        let exponent_f32 = t.exponent - 15 + 127;
        let fraction_f32: u32 = (t.fraction as u32) << 13;
        let bitvalue_f32: u32 = fraction_f32 | ((exponent_f32 as u32) << 23) | ((positive_f32 as u32) << 31);
        let value = unsafe { transmute::<u32, f32>(bitvalue_f32) };
        return value;        
    }
}

impl From<Float32> for f32 {
    fn from(t: Float32) -> f32 {
        t.value
    }
}

impl From<Float64> for f64 {
    fn from(t: Float64) -> f64 {
        t.value
    }
}

impl UavcanSerializable for Bool {
    fn uavcan_size(&self) -> usize {
        1
    }
}

impl UavcanSerializable for IntX {
    fn uavcan_size(&self) -> usize {
        self.x
    }
}

impl UavcanSerializable for UintX {
    fn uavcan_size(&self) -> usize {
        self.x
    }
}

impl UavcanSerializable for Float16 {
    fn uavcan_size(&self) -> usize {
        16
    }
}

impl UavcanSerializable for Float32 {
    fn uavcan_size(&self) -> usize {
        32
    }
}

impl UavcanSerializable for Float64 {
    fn uavcan_size(&self) -> usize {
        64
    }
}

impl UavcanSerializable for VoidX {
    fn uavcan_size(&self) -> usize {
        self.x
    }
}
