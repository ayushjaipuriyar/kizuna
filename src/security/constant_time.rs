/// Constant-time cryptographic operations to prevent timing attacks
/// 
/// This module provides utilities for performing cryptographic operations
/// in constant time, preventing side-channel attacks that could leak
/// sensitive information through timing variations.

/// Constant-time operations for cryptographic security
pub struct ConstantTime;

impl ConstantTime {
    /// Compare two byte slices in constant time
    /// 
    /// This function takes the same amount of time regardless of where
    /// the first difference occurs, preventing timing attacks.
    /// 
    /// Returns true if the slices are equal, false otherwise.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use kizuna::security::constant_time::ConstantTime;
    /// 
    /// let a = b"secret";
    /// let b = b"secret";
    /// let c = b"public";
    /// 
    /// assert!(ConstantTime::compare(a, b));
    /// assert!(!ConstantTime::compare(a, c));
    /// ```
    pub fn compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            // Even for length mismatch, we do a constant-time operation
            // to avoid leaking information about the expected length
            return Self::compare_fixed_length(&[0u8; 32], &[1u8; 32]);
        }
        
        Self::compare_fixed_length(a, b)
    }
    
    /// Compare two byte slices of the same length in constant time
    /// 
    /// This is an internal helper that assumes both slices have the same length.
    fn compare_fixed_length(a: &[u8], b: &[u8]) -> bool {
        let mut result = 0u8;
        
        // XOR all bytes and accumulate the result
        // This ensures we touch every byte regardless of where differences occur
        for (byte_a, byte_b) in a.iter().zip(b.iter()) {
            result |= byte_a ^ byte_b;
        }
        
        // Use constant-time comparison for the final result
        Self::is_zero_u8(result)
    }
    
    /// Check if a u8 value is zero in constant time
    /// 
    /// This avoids branching that could leak information through timing.
    #[inline(always)]
    fn is_zero_u8(value: u8) -> bool {
        // Convert to u32 to avoid overflow
        let v = value as u32;
        // If value is 0, this will be 0xFFFFFFFF, otherwise 0x00000000
        let result = ((v | (!v).wrapping_add(1)) >> 31).wrapping_sub(1);
        result == 0xFFFFFFFF
    }
    
    /// Select between two values in constant time based on a condition
    /// 
    /// If condition is true (non-zero), returns a, otherwise returns b.
    /// This operation takes the same time regardless of the condition value.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use kizuna::security::constant_time::ConstantTime;
    /// 
    /// let a = 42u8;
    /// let b = 17u8;
    /// 
    /// assert_eq!(ConstantTime::select_u8(true, a, b), a);
    /// assert_eq!(ConstantTime::select_u8(false, a, b), b);
    /// ```
    pub fn select_u8(condition: bool, a: u8, b: u8) -> u8 {
        let mask = (condition as u8).wrapping_neg();
        (a & mask) | (b & !mask)
    }
    
    /// Select between two u32 values in constant time
    pub fn select_u32(condition: bool, a: u32, b: u32) -> u32 {
        let mask = (condition as u32).wrapping_neg();
        (a & mask) | (b & !mask)
    }
    
    /// Select between two u64 values in constant time
    pub fn select_u64(condition: bool, a: u64, b: u64) -> u64 {
        let mask = (condition as u64).wrapping_neg();
        (a & mask) | (b & !mask)
    }
    
    /// Copy data from source to destination in constant time
    /// 
    /// This ensures the copy operation is not optimized away and takes
    /// constant time regardless of the data values.
    pub fn copy(src: &[u8], dst: &mut [u8]) {
        assert_eq!(src.len(), dst.len(), "Source and destination must have same length");
        
        // Use volatile operations to prevent compiler optimization
        for i in 0..src.len() {
            unsafe {
                std::ptr::write_volatile(&mut dst[i], src[i]);
            }
        }
    }
    
    /// Conditionally copy data in constant time
    /// 
    /// If condition is true, copies src to dst. Otherwise, dst is unchanged.
    /// This operation takes the same time regardless of the condition.
    pub fn conditional_copy(condition: bool, src: &[u8], dst: &mut [u8]) {
        assert_eq!(src.len(), dst.len(), "Source and destination must have same length");
        
        for i in 0..src.len() {
            dst[i] = Self::select_u8(condition, src[i], dst[i]);
        }
    }
    
    /// Check if a byte slice is all zeros in constant time
    pub fn is_zero(data: &[u8]) -> bool {
        let mut result = 0u8;
        
        for &byte in data {
            result |= byte;
        }
        
        Self::is_zero_u8(result)
    }
    
    /// Compare two 32-byte arrays in constant time
    /// 
    /// This is optimized for common cryptographic key sizes.
    pub fn compare_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
        let mut result = 0u8;
        
        for i in 0..32 {
            result |= a[i] ^ b[i];
        }
        
        Self::is_zero_u8(result)
    }
    
    /// Compare two 64-byte arrays in constant time
    /// 
    /// This is optimized for larger cryptographic values like signatures.
    pub fn compare_64(a: &[u8; 64], b: &[u8; 64]) -> bool {
        let mut result = 0u8;
        
        for i in 0..64 {
            result |= a[i] ^ b[i];
        }
        
        Self::is_zero_u8(result)
    }
    
    /// Constant-time less-than comparison for u32
    /// 
    /// Returns true if a < b, false otherwise.
    /// This operation takes constant time regardless of the values.
    pub fn less_than_u32(a: u32, b: u32) -> bool {
        // Compute (a - b) and check the sign bit
        let diff = a.wrapping_sub(b);
        let sign_bit = (diff >> 31) & 1;
        sign_bit == 1
    }
    
    /// Constant-time less-than comparison for u64
    pub fn less_than_u64(a: u64, b: u64) -> bool {
        let diff = a.wrapping_sub(b);
        let sign_bit = (diff >> 63) & 1;
        sign_bit == 1
    }
    
    /// Constant-time equality check for u32
    pub fn equal_u32(a: u32, b: u32) -> bool {
        let diff = a ^ b;
        let result = ((diff | (!diff).wrapping_add(1)) >> 31).wrapping_sub(1);
        result == 0xFFFFFFFF
    }
    
    /// Constant-time equality check for u64
    pub fn equal_u64(a: u64, b: u64) -> bool {
        let diff = a ^ b;
        let result = ((diff | (!diff).wrapping_add(1)) >> 63).wrapping_sub(1);
        result == 0xFFFFFFFFFFFFFFFF
    }
}

/// Trait for types that support constant-time comparison
pub trait ConstantTimeEq {
    /// Compare two values in constant time
    fn ct_eq(&self, other: &Self) -> bool;
}

impl ConstantTimeEq for [u8; 32] {
    fn ct_eq(&self, other: &Self) -> bool {
        ConstantTime::compare_32(self, other)
    }
}

impl ConstantTimeEq for [u8; 64] {
    fn ct_eq(&self, other: &Self) -> bool {
        ConstantTime::compare_64(self, other)
    }
}

impl ConstantTimeEq for &[u8] {
    fn ct_eq(&self, other: &Self) -> bool {
        ConstantTime::compare(self, other)
    }
}

impl ConstantTimeEq for Vec<u8> {
    fn ct_eq(&self, other: &Self) -> bool {
        ConstantTime::compare(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constant_time_compare() {
        let a = b"secret key";
        let b = b"secret key";
        let c = b"public key";
        
        assert!(ConstantTime::compare(a, b));
        assert!(!ConstantTime::compare(a, c));
        
        // Different lengths
        assert!(!ConstantTime::compare(a, b"short"));
    }
    
    #[test]
    fn test_compare_32() {
        let a = [42u8; 32];
        let b = [42u8; 32];
        let mut c = [42u8; 32];
        c[31] = 43;
        
        assert!(ConstantTime::compare_32(&a, &b));
        assert!(!ConstantTime::compare_32(&a, &c));
    }
    
    #[test]
    fn test_compare_64() {
        let a = [17u8; 64];
        let b = [17u8; 64];
        let mut c = [17u8; 64];
        c[0] = 18;
        
        assert!(ConstantTime::compare_64(&a, &b));
        assert!(!ConstantTime::compare_64(&a, &c));
    }
    
    #[test]
    fn test_select_u8() {
        let a = 42u8;
        let b = 17u8;
        
        assert_eq!(ConstantTime::select_u8(true, a, b), a);
        assert_eq!(ConstantTime::select_u8(false, a, b), b);
    }
    
    #[test]
    fn test_select_u32() {
        let a = 12345u32;
        let b = 67890u32;
        
        assert_eq!(ConstantTime::select_u32(true, a, b), a);
        assert_eq!(ConstantTime::select_u32(false, a, b), b);
    }
    
    #[test]
    fn test_select_u64() {
        let a = 123456789u64;
        let b = 987654321u64;
        
        assert_eq!(ConstantTime::select_u64(true, a, b), a);
        assert_eq!(ConstantTime::select_u64(false, a, b), b);
    }
    
    #[test]
    fn test_is_zero() {
        assert!(ConstantTime::is_zero(&[0u8; 32]));
        assert!(!ConstantTime::is_zero(&[0, 0, 0, 1]));
        assert!(!ConstantTime::is_zero(&[1, 0, 0, 0]));
    }
    
    #[test]
    fn test_constant_time_copy() {
        let src = b"sensitive data";
        let mut dst = [0u8; 14];
        
        ConstantTime::copy(src, &mut dst);
        assert_eq!(&dst, src);
    }
    
    #[test]
    fn test_conditional_copy() {
        let src = b"new data";
        let mut dst = *b"old data";
        
        // Copy when condition is true
        ConstantTime::conditional_copy(true, src, &mut dst);
        assert_eq!(&dst, src);
        
        // Don't copy when condition is false
        let original = dst;
        ConstantTime::conditional_copy(false, b"ignored!", &mut dst);
        assert_eq!(&dst, &original);
    }
    
    #[test]
    fn test_less_than_u32() {
        assert!(ConstantTime::less_than_u32(10, 20));
        assert!(!ConstantTime::less_than_u32(20, 10));
        assert!(!ConstantTime::less_than_u32(15, 15));
    }
    
    #[test]
    fn test_less_than_u64() {
        assert!(ConstantTime::less_than_u64(100, 200));
        assert!(!ConstantTime::less_than_u64(200, 100));
        assert!(!ConstantTime::less_than_u64(150, 150));
    }
    
    #[test]
    fn test_equal_u32() {
        assert!(ConstantTime::equal_u32(42, 42));
        assert!(!ConstantTime::equal_u32(42, 43));
    }
    
    #[test]
    fn test_equal_u64() {
        assert!(ConstantTime::equal_u64(12345, 12345));
        assert!(!ConstantTime::equal_u64(12345, 12346));
    }
    
    #[test]
    fn test_constant_time_eq_trait() {
        let a = [1u8; 32];
        let b = [1u8; 32];
        let c = [2u8; 32];
        
        assert!(a.ct_eq(&b));
        assert!(!a.ct_eq(&c));
    }
}
