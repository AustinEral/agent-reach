use agent_id::RootKey;
use base64::Engine;

fn main() {
    // Generate a test key
    let key = RootKey::generate();
    let did = key.did().to_string();
    let endpoint = "wss://test.example.com:8080";
    let ttl = 3600u64;
    
    // Create message to sign: did:endpoint:ttl
    let message = format!("{}:{}:{}", did, endpoint, ttl);
    
    // Sign it
    let signature = key.sign(message.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    
    // Output JSON for curl
    println!("{{\"did\":\"{}\",\"endpoint\":\"{}\",\"ttl\":{},\"signature\":\"{}\"}}", 
             did, endpoint, ttl, sig_b64);
}
