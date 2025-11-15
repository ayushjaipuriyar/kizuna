/// Demo of secure memory management and constant-time operations
/// 
/// This example demonstrates the secure memory utilities and constant-time
/// cryptographic operations implemented for the security system.

use kizuna::security::secure_memory::{SecureBuffer, SecureKey, SecureMemory};
use kizuna::security::constant_time::ConstantTime;

fn main() {
    println!("=== Secure Memory Management Demo ===\n");
    
    // Demonstrate SecureBuffer
    println!("1. SecureBuffer - Automatic zeroization");
    {
        let mut buffer = SecureBuffer::from_slice(b"sensitive data");
        println!("   Buffer created with: {:?}", std::str::from_utf8(buffer.as_slice()).unwrap());
        println!("   Buffer length: {}", buffer.len());
        
        buffer.extend_from_slice(b" - more data");
        println!("   After extend: {:?}", std::str::from_utf8(buffer.as_slice()).unwrap());
        
        // Buffer will be automatically zeroized when it goes out of scope
    }
    println!("   Buffer dropped and zeroized\n");
    
    // Demonstrate SecureKey
    println!("2. SecureKey - Protected cryptographic keys");
    {
        let key = SecureKey::<32>::new([42u8; 32]);
        println!("   Key created (32 bytes)");
        println!("   First 8 bytes: {:?}", &key.as_bytes()[..8]);
        
        // Key will be automatically zeroized when it goes out of scope
    }
    println!("   Key dropped and zeroized\n");
    
    // Demonstrate constant-time comparison
    println!("3. Constant-time comparison");
    let secret1 = b"my_secret_key_123";
    let secret2 = b"my_secret_key_123";
    let secret3 = b"different_key_456";
    
    println!("   Comparing equal secrets: {}", ConstantTime::compare(secret1, secret2));
    println!("   Comparing different secrets: {}", ConstantTime::compare(secret1, secret3));
    println!("   (Both comparisons take constant time)\n");
    
    // Demonstrate secure copy
    println!("4. Secure copy (prevents compiler optimization)");
    let src = b"sensitive";
    let mut dst = [0u8; 9];
    SecureMemory::secure_copy(src, &mut dst);
    println!("   Copied: {:?}", std::str::from_utf8(&dst).unwrap());
    
    // Zeroize the destination
    SecureMemory::secure_zeroize(&mut dst);
    println!("   After zeroize: {:?}\n", dst);
    
    // Demonstrate random key generation
    println!("5. Random key generation");
    let random_key = SecureMemory::random_key::<32>();
    println!("   Generated random key (first 8 bytes): {:?}", &random_key.as_bytes()[..8]);
    
    let random_buffer = SecureMemory::random_buffer(16);
    println!("   Generated random buffer (16 bytes): {:?}\n", random_buffer.as_slice());
    
    // Demonstrate constant-time selection
    println!("6. Constant-time conditional selection");
    let a = 42u8;
    let b = 17u8;
    println!("   Select a (42) if true: {}", ConstantTime::select_u8(true, a, b));
    println!("   Select b (17) if false: {}", ConstantTime::select_u8(false, a, b));
    println!("   (Selection takes constant time regardless of condition)\n");
    
    // Demonstrate constant-time equality
    println!("7. Constant-time equality checks");
    let val1 = 12345u32;
    let val2 = 12345u32;
    let val3 = 67890u32;
    println!("   12345 == 12345: {}", ConstantTime::equal_u32(val1, val2));
    println!("   12345 == 67890: {}", ConstantTime::equal_u32(val1, val3));
    println!("   (Comparison takes constant time)\n");
    
    println!("=== Demo Complete ===");
    println!("\nAll sensitive data has been automatically zeroized.");
    println!("All cryptographic operations were performed in constant time.");
}
