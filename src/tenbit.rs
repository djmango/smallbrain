pub fn scale_to_10_bits(samples: &[i16]) -> Vec<u16> {
    samples
        .iter()
        .map(|&sample| ((sample as u32 - i16::MIN as u32) >> 6) as u16) // Normalize to 10-bit range
        .collect()
}

pub fn scale_from_10_bits(samples: &[u16]) -> Vec<i16> {
    samples
        .iter()
        // .map(|&sample| (sample as i16) << 6) // Denormalize from 10-bit range
        .map(|&sample| (((sample as i32) << 6) + i16::MIN as i32) as i16)
        .collect()

    // // Scale back to 16-bit
    //     let scaled_back_samples = unpacked_samples
    //         .iter()
    //         .map(|&sample| (((sample as i32) << 6) + i16::MIN as i32) as i16)
    //         .collect();
}

pub fn pack_10_bit_values(samples: &[u16]) -> Vec<i16> {
    let mut packed = Vec::with_capacity((samples.len() * 10 + 15) / 16); // Reserve space
    let mut buffer = 0u32;
    let mut bits_in_buffer = 0;

    for &sample in samples {
        buffer |= (sample as u32) << bits_in_buffer;
        bits_in_buffer += 10;
        while bits_in_buffer >= 16 {
            packed.push((buffer as u16) & 0xFFFF);
            buffer >>= 16;
            bits_in_buffer -= 16;
        }
    }

    if bits_in_buffer > 0 {
        packed.push(buffer as u16);
    }

    // To i16
    let packed_i16: Vec<i16> = packed.iter().map(|&x| x as i16).collect();

    // packed
    packed_i16
}

pub fn unpack_10_bit_values(packed: &[u16]) -> Vec<u16> {
    let mut samples = Vec::with_capacity(packed.len() * 16 / 10);
    let mut buffer = 0u32;
    let mut bits_in_buffer = 0;

    for &value in packed {
        buffer |= (value as u32) << bits_in_buffer;
        bits_in_buffer += 16;
        while bits_in_buffer >= 10 {
            samples.push((buffer & 0x3FF) as u16);
            buffer >>= 10;
            bits_in_buffer -= 10;
        }
    }

    samples
}
