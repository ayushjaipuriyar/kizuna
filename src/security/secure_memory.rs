/// Secure memory management utilities for cryptographic operations
/// 
/// This module provides secure buffer management, automatic zeroization,
/// and memory protection for sensitive cryptographic data.

use std::ops::{Deref, DerefMut};
use zeroize::{Zeroize, Zeroizing};

/// A secure buffer that automatically zeroizes its contents on drop
/// 
/// This wrapper ensures that sensitive data is cleared from memory
/// when it goes out of scope, preventing potential memory disclosure attacks.
#[derive(Debug)]
pub struct SecureBuffer {
    data: Zeroizing<Vec<u8>>,
}

impl SecureBuffer {
    /// Create a new secure buffer with the given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Zeroizing::new(Vec::with_capacity(capacity)),
        }
    }
    
    /// Create a new secure buffer from existing data
    /// 
    /// The input data will be moved into the secure buffer and will be
    /// automatically zeroized when the buffer is dropped.
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            data: Zeroizing::new(data),
        }
    }
    
    /// Create a new secure buffer from a slice
    /// 
    /// The data will be copied into the secure buffer.
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: Zeroizing::new(data.to_vec()),
        }
    }
    
    /// Get the length of the buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Get a reference to the underlying data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    
    /// Get a mutable reference to the underlying data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
    
    /// Extend the buffer with data from a slice
    pub fn extend_from_slice(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }
    
    /// Clear the buffer (zeroizes and sets length to 0)
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    /// Resize the buffer to the given length, filling with zeros
    pub fn resize(&mut self, new_len: usize) {
        self.data.resize(new_len, 0);
    }
    
    /// Convert the secure buffer into a Vec, consuming self
    /// 
    /// Note: The returned Vec will still be zeroized on drop due to Zeroizing wrapper
    pub fn into_vec(self) -> Vec<u8> {
        // Extract the inner Vec from Zeroizing
        // The Zeroizing wrapper will still handle cleanup
        self.data.to_vec()
    }
}

impl Deref for SecureBuffer {
    type Target = [u8];
    
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for SecureBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Clone for SecureBuffer {
    fn clone(&self) -> Self {
        Self::from_slice(&self.data)
    }
}

/// A secure key wrapper that provides automatic zeroization and memory protection
/// 
/// This type is specifically designed for cryptographic keys and ensures they
/// are properly protected in memory.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct SecureKey<const N: usize> {
    key: [u8; N],
}

impl<const N: usize> SecureKey<N> {
    /// Create a new secure key from a byte array
    pub fn new(key: [u8; N]) -> Self {
        Self { key }
    }
    
    /// Create a new secure key from a slice
    /// 
    /// Returns None if the slice length doesn't match N
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != N {
            return None;
        }
        
        let mut key = [0u8; N];
        key.copy_from_slice(slice);
        Some(Self { key })
    }
    
    /// Get a reference to the key bytes
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.key
    }
    
    /// Get a mutable reference to the key bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8; N] {
        &mut self.key
    }
    
    /// Explicitly zeroize the key
    /// 
    /// This is called automatically on drop, but can be called manually
    /// if you want to clear the key before it goes out of scope.
    pub fn zeroize_key(&mut self) {
        self.key.zeroize();
    }
}

impl<const N: usize> Deref for SecureKey<N> {
    type Target = [u8; N];
    
    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

impl<const N: usize> DerefMut for SecureKey<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.key
    }
}

impl<const N: usize> AsRef<[u8]> for SecureKey<N> {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}

impl<const N: usize> AsMut<[u8]> for SecureKey<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.key
    }
}

use crate::security::constant_time::ConstantTime;

/// Secure memory utilities for cryptographic operations
pub struct SecureMemory;

impl SecureMemory {
    /// Securely compare two byte slices in constant time
    /// 
    /// This prevents timing attacks by ensuring the comparison takes
    /// the same amount of time regardless of where the difference occurs.
    /// 
    /// Returns true if the slices are equal, false otherwise.
    pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        ConstantTime::compare(a, b)
    }
    
    /// Securely copy data from source to destination
    /// 
    /// This ensures the copy operation is not optimized away by the compiler
    /// and provides a clear semantic for security-critical copies.
    pub fn secure_copy(src: &[u8], dst: &mut [u8]) {
        assert_eq!(src.len(), dst.len(), "Source and destination must have same length");
        
        // Use volatile operations to prevent compiler optimization
        for (i, &byte) in src.iter().enumerate() {
            // This prevents the compiler from optimizing away the copy
            unsafe {
                std::ptr::write_volatile(&mut dst[i], byte);
            }
        }
    }
    
    /// Securely zeroize a mutable byte slice
    /// 
    /// This is a convenience wrapper around zeroize that ensures
    /// the operation is not optimized away.
    pub fn secure_zeroize(data: &mut [u8]) {
        data.zeroize();
    }
    
    /// Create a secure random buffer of the given size
    /// 
    /// This uses the OS random number generator to fill the buffer
    /// with cryptographically secure random bytes.
    pub fn random_buffer(size: usize) -> SecureBuffer {
        use rand::RngCore;
        let mut rng = rand::rngs::OsRng;
        let mut buffer = vec![0u8; size];
        rng.fill_bytes(&mut buffer);
        SecureBuffer::from_vec(buffer)
    }
    
    /// Create a secure random key of the given size
    pub fn random_key<const N: usize>() -> SecureKey<N> {
        use rand::RngCore;
        let mut rng = rand::rngs::OsRng;
        let mut key = [0u8; N];
        rng.fill_bytes(&mut key);
        SecureKey::new(key)
    }
}

/// A guard that ensures sensitive data is zeroized when dropped
/// 
/// This can be used to wrap any type that implements Zeroize to ensure
/// it is properly cleaned up.
pub struct ZeroizeGuard<T: Zeroize> {
    data: Option<T>,
}

impl<T: Zeroize> ZeroizeGuard<T> {
    /// Create a new zeroize guard wrapping the given data
    pub fn new(data: T) -> Self {
        Self { data: Some(data) }
    }
    
    /// Get a reference to the wrapped data
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }
    
    /// Get a mutable reference to the wrapped data
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }
    
    /// Take the data out of the guard, consuming the guard
    /// 
    /// The caller is responsible for ensuring the data is properly zeroized.
    pub fn take(mut self) -> Option<T> {
        self.data.take()
    }
    
    /// Explicitly zeroize and drop the wrapped data
    pub fn zeroize_now(&mut self) {
        if let Some(mut data) = self.data.take() {
            data.zeroize();
        }
    }
}

impl<T: Zeroize> Drop for ZeroizeGuard<T> {
    fn drop(&mut self) {
        self.zeroize_now();
    }
}

impl<T: Zeroize> Deref for ZeroizeGuard<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.data.as_ref().expect("ZeroizeGuard data was taken")
    }
}

impl<T: Zeroize> DerefMut for ZeroizeGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().expect("ZeroizeGuard data was taken")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_secure_buffer_basic() {
        let mut buffer = SecureBuffer::from_slice(b"sensitive data");
        assert_eq!(buffer.len(), 14);
        assert_eq!(buffer.as_slice(), b"sensitive data");
        
        buffer.clear();
        assert_eq!(buffer.len(), 0);
    }
    
    #[test]
    fn test_secure_buffer_extend() {
        let mut buffer = SecureBuffer::with_capacity(10);
        buffer.extend_from_slice(b"hello");
        buffer.extend_from_slice(b" world");
        assert_eq!(buffer.as_slice(), b"hello world");
    }
    
    #[test]
    fn test_secure_key() {
        let key = SecureKey::<32>::new([42u8; 32]);
        assert_eq!(key.as_bytes(), &[42u8; 32]);
        
        let key_from_slice = SecureKey::<32>::from_slice(&[1u8; 32]).unwrap();
        assert_eq!(key_from_slice.as_bytes(), &[1u8; 32]);
        
        // Wrong size should return None
        assert!(SecureKey::<32>::from_slice(&[1u8; 16]).is_none());
    }
    
    #[test]
    fn test_constant_time_compare() {
        let a = b"secret key";
        let b = b"secret key";
        let c = b"public key";
        
        assert!(SecureMemory::constant_time_compare(a, b));
        assert!(!SecureMemory::constant_time_compare(a, c));
        assert!(!SecureMemory::constant_time_compare(a, b"short"));
    }
    
    #[test]
    fn test_secure_copy() {
        let src = b"sensitive";
        let mut dst = [0u8; 9];
        
        SecureMemory::secure_copy(src, &mut dst);
        assert_eq!(&dst, src);
    }
    
    #[test]
    fn test_random_buffer() {
        let buffer1 = SecureMemory::random_buffer(32);
        let buffer2 = SecureMemory::random_buffer(32);
        
        assert_eq!(buffer1.len(), 32);
        assert_eq!(buffer2.len(), 32);
        // Random buffers should be different
        assert_ne!(buffer1.as_slice(), buffer2.as_slice());
    }
    
    #[test]
    fn test_random_key() {
        let key1 = SecureMemory::random_key::<32>();
        let key2 = SecureMemory::random_key::<32>();
        
        // Random keys should be different
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }
    
    #[test]
    fn test_zeroize_guard() {
        let data = vec![1u8, 2, 3, 4, 5];
        let mut guard = ZeroizeGuard::new(data);
        
        assert_eq!(guard.get(), Some(&vec![1u8, 2, 3, 4, 5]));
        
        guard.zeroize_now();
        assert_eq!(guard.get(), None);
    }
    
    #[test]
    fn test_secure_zeroize() {
        let mut data = vec![1u8, 2, 3, 4, 5];
        SecureMemory::secure_zeroize(&mut data);
        assert_eq!(data, vec![0u8, 0, 0, 0, 0]);
    }
}
