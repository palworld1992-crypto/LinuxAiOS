with Interfaces.C;
with System;

-- SPARK_Mode => On giúp kiểm chứng hình thức (formal verification)
-- Lưu ý: Các hàm xuất C ABI được khai báo là procedure và export bằng pragma Export.
package Crypto_Engine with SPARK_Mode => On is

   use type Interfaces.C.size_t;

   --------------------------------------------------------------------
   -- ĐỊNH NGHĨA TYPE (Sử dụng char_array cho dữ liệu nhị phân)
   --------------------------------------------------------------------
   subtype Key_32 is Interfaces.C.char_array (0 .. 31);
   subtype Key_64 is Interfaces.C.char_array (0 .. 63);
   subtype Key_12 is Interfaces.C.char_array (0 .. 11);
   
   -- Kyber-768 (KEM) 
   subtype Key_1568 is Interfaces.C.char_array (0 .. 1567);
   subtype Key_2400 is Interfaces.C.char_array (0 .. 2399);
   subtype Key_1312 is Interfaces.C.char_array (0 .. 1311);
   
   -- ML-DSA-65 / Dilithium3
   subtype Key_1952 is Interfaces.C.char_array (0 .. 1951);
   subtype Key_4032 is Interfaces.C.char_array (0 .. 4031);
   subtype Key_3309 is Interfaces.C.char_array (0 .. 3308);

    --------------------------------------------------------------------
    -- AES-GCM 256
    -- FIX: Caller cấp phát output buffer, tránh cross-library free
    -- Max output = plaintext_len + 16 (tag)
    --------------------------------------------------------------------
    procedure AES_GCM_Encrypt
      (Key             : Key_32;
       Plaintext       : System.Address;
       Plaintext_Len   : Interfaces.C.size_t;
       AAD             : System.Address;
       AAD_Len         : Interfaces.C.size_t;
       Ciphertext_Buf  : System.Address;
       Ciphertext_Buf_Size : Interfaces.C.size_t;
       Ciphertext_Len  : out Interfaces.C.size_t;
       Status          : out Interfaces.C.int)
      with Export,
           Convention => C,
           External_Name => "crypto_engine__aes_gcm_encrypt",
           Depends => (Status => (Key, Plaintext, Plaintext_Len, AAD, AAD_Len, Ciphertext_Buf, Ciphertext_Buf_Size),
                       Ciphertext_Len => (Key, Plaintext, Plaintext_Len, AAD, AAD_Len));

    procedure AES_GCM_Decrypt
      (Key             : Key_32;
       Ciphertext      : System.Address;
       Ciphertext_Len  : Interfaces.C.size_t;
       AAD             : System.Address;
       AAD_Len         : Interfaces.C.size_t;
       Plaintext_Buf   : System.Address;
       Plaintext_Buf_Size : Interfaces.C.size_t;
       Plaintext_Len   : out Interfaces.C.size_t;
       Status          : out Interfaces.C.int)
      with Export,
           Convention => C,
           External_Name => "crypto_engine__aes_gcm_decrypt",
           Depends => (Status => (Key, Ciphertext, Ciphertext_Len, AAD, AAD_Len, Plaintext_Buf, Plaintext_Buf_Size),
                       Plaintext_Len => (Key, Ciphertext, Ciphertext_Len, AAD, AAD_Len));

    --------------------------------------------------------------------
    -- Post-Quantum Signature (Dilithium3)
    -- FIX: Caller cấp phát signature buffer, tránh cross-library free
    -- FIX: Use System.Address for keys to avoid copy on pass
    -- FIX: External_Name thống nhất prefix crypto_engine__ như Kyber/HMAC/AES
    --------------------------------------------------------------------
     procedure Dilithium_Keypair
       (Public_Key  : System.Address;
        Secret_Key  : System.Address;
        Status      : out Interfaces.C.int)
       with Export,
            Convention => C,
            External_Name => "crypto_engine__dilithium_keypair",
            Depends => (Status => (Public_Key, Secret_Key));

     procedure Dilithium_Sign
       (Secret_Key     : System.Address;
        Message        : System.Address;
        Message_Len    : Interfaces.C.size_t;
        Signature_Buf  : System.Address;
        Signature_Buf_Size : Interfaces.C.size_t;
        Signature_Len_Ptr : System.Address;  -- Address of size_t
        Status         : out Interfaces.C.int)
       with Export,
            Convention => C,
            External_Name => "crypto_engine__dilithium_sign",
            Depends => (Status => (Secret_Key, Message, Message_Len, Signature_Buf, Signature_Buf_Size, Signature_Len_Ptr));

     procedure Dilithium_Verify
       (Public_Key   : System.Address;
        Message      : System.Address;
        Message_Len  : Interfaces.C.size_t;
        Signature    : System.Address;
        Signature_Len : Interfaces.C.size_t;
        Status_Ptr   : System.Address)  -- Changed to Address for proper C ABI
       with Export,
            Convention => C,
            External_Name => "crypto_engine__dilithium_verify",
            Depends => (null => (Public_Key, Message, Message_Len, Signature, Signature_Len, Status_Ptr));

 end Crypto_Engine;
