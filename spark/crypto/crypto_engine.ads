pragma Style_Checks (Off);
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
   --------------------------------------------------------------------
   procedure AES_GCM_Encrypt
     (Key            : Key_32;
      Plaintext      : System.Address;
      Plaintext_Len  : Interfaces.C.size_t;
      AAD            : System.Address;
      AAD_Len        : Interfaces.C.size_t;
      Ciphertext_Out : out System.Address;
      Ciphertext_Len : out Interfaces.C.size_t;
      Status         : out Interfaces.C.int)
     with Depends => (Status => (Key, Plaintext, Plaintext_Len, AAD, AAD_Len),
                      Ciphertext_Out => (Key, Plaintext, Plaintext_Len, AAD, AAD_Len),
                      Ciphertext_Len => (Key, Plaintext, Plaintext_Len, AAD, AAD_Len));

   procedure AES_GCM_Decrypt
     (Key            : Key_32;
      Ciphertext     : System.Address;
      Ciphertext_Len : Interfaces.C.size_t;
      AAD            : System.Address;
      AAD_Len        : Interfaces.C.size_t;
      Plaintext_Out  : out System.Address;
      Plaintext_Len  : out Interfaces.C.size_t;
      Status         : out Interfaces.C.int)
     with Depends => (Status => (Key, Ciphertext, Ciphertext_Len, AAD, AAD_Len),
                      Plaintext_Out => (Key, Ciphertext, Ciphertext_Len, AAD, AAD_Len),
                      Plaintext_Len => (Key, Ciphertext, Ciphertext_Len, AAD, AAD_Len));

   --------------------------------------------------------------------
   -- HMAC-SHA256
   --------------------------------------------------------------------
   procedure HMAC_SHA256
     (Key      : Key_32;
      Data     : System.Address;
      Data_Len : Interfaces.C.size_t;
      MAC_Out  : out Key_32;
      Status   : out Interfaces.C.int)
     with Depends => (Status => (Key, Data, Data_Len),
                      MAC_Out => (Key, Data, Data_Len));

   --------------------------------------------------------------------
   -- Post-Quantum KEM (Kyber-768)
   --------------------------------------------------------------------
   procedure Kyber_Keypair
     (Public_Key : out Key_1568;
      Secret_Key : out Key_2400;
      Status     : out Interfaces.C.int)
     with Depends => (Status | Public_Key | Secret_Key => null);

   procedure Kyber_Encaps
     (Public_Key    : Key_1568;
      Ciphertext    : out Key_1312;
      Shared_Secret : out Key_32;
      Status        : out Interfaces.C.int)
     with Depends => (Status => Public_Key,
                      Ciphertext => Public_Key,
                      Shared_Secret => Public_Key);

   procedure Kyber_Decaps
     (Secret_Key    : Key_2400;
      Ciphertext    : Key_1312;
      Shared_Secret : out Key_32;
      Status        : out Interfaces.C.int)
     with Depends => (Status => (Secret_Key, Ciphertext),
                      Shared_Secret => (Secret_Key, Ciphertext));

   --------------------------------------------------------------------
   -- Post-Quantum Signature (Dilithium3)
   --------------------------------------------------------------------
   procedure Dilithium_Keypair 
     (Public_Key : out Key_1952; 
      Secret_Key : out Key_4032;
      Status     : out Interfaces.C.int)
     with Depends => (Status | Public_Key | Secret_Key => null);

   procedure Dilithium_Sign
     (Secret_Key   : Key_4032;
      Message      : System.Address;
      Message_Len  : Interfaces.C.size_t;
      Signature    : out Key_3309;
      Status       : out Interfaces.C.int)
     with Depends => (Status => (Secret_Key, Message, Message_Len),
                      Signature => (Secret_Key, Message, Message_Len));

   procedure Dilithium_Verify
     (Public_Key   : Key_1952;
      Message      : System.Address;
      Message_Len  : Interfaces.C.size_t;
      Signature    : Key_3309;
      Status       : out Interfaces.C.int)
     with Depends => (Status => (Public_Key, Message, Message_Len, Signature));

   -- Giải phóng bộ nhớ heap được cấp phát bên phía C
   procedure Crypto_Free_Buffer (Ptr : System.Address)
     with Depends => (null => Ptr);

private
    
end Crypto_Engine;