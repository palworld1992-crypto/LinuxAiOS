pragma Style_Checks (Off);

with System.Storage_Elements;
with Interfaces.C.Strings;
with Ada.Unchecked_Conversion;

package body Crypto_Engine is

   -- 1. Định nghĩa hàm chuyển đổi chuẩn Enterprise (Internal use only)
   -- Chuyển đổi từ chars_ptr (chuỗi C) sang System.Address để truyền vào OSSL_PARAM
   function To_Addr is new Ada.Unchecked_Conversion 
     (Source => Interfaces.C.Strings.chars_ptr, 
      Target => System.Address);

   -- Sử dụng các kiểu dữ liệu bổ trợ cho tính toán con trỏ và logic
   use type System.Address;
   use type Interfaces.C.int;
   use type Interfaces.C.Strings.chars_ptr;

   -- 2. Tên thuật toán liboqs (ASCII.NUL là bắt buộc khi truyền vào C)
   Kyber_Name_Str     : constant String := "Kyber768" & ASCII.NUL;
   Dilithium_Name_Str : constant String := "ML-DSA-65" & ASCII.NUL;

   --------------------------------------------------------------------
   -- Import quản lý bộ nhớ từ thư viện C chuẩn
   --------------------------------------------------------------------
   procedure C_Free (Ptr : System.Address)
     with Import, Convention => C, External_Name => "free";

   function C_Malloc (Size : Interfaces.C.size_t) return System.Address
     with Import, Convention => C, External_Name => "malloc";

   --------------------------------------------------------------------
   -- Định nghĩa cấu trúc OSSL_PARAM (Bắt buộc cho OpenSSL 3.x)
   --------------------------------------------------------------------
   type OSSL_PARAM is record
      Key         : Interfaces.C.Strings.chars_ptr;
      Data_Type   : Interfaces.C.unsigned;
      Data        : System.Address;
      Data_Size   : Interfaces.C.size_t;
      Return_Size : Interfaces.C.size_t;
   end record
     with Convention => C;

   --------------------------------------------------------------------
   -- Import OpenSSL EVP (Cipher Management)
   --------------------------------------------------------------------
   function EVP_CIPHER_fetch 
     (libctx : System.Address; name : Interfaces.C.Strings.chars_ptr; propq : System.Address) return System.Address
     with Import, Convention => C, External_Name => "EVP_CIPHER_fetch";

   function EVP_CIPHER_CTX_new return System.Address
     with Import, Convention => C, External_Name => "EVP_CIPHER_CTX_new";

   procedure EVP_CIPHER_CTX_free (ctx : System.Address)
     with Import, Convention => C, External_Name => "EVP_CIPHER_CTX_free";

   function EVP_CIPHER_CTX_ctrl
     (ctx  : System.Address; 
      cmd  : Interfaces.C.int; 
      p1   : Interfaces.C.int; 
      p2   : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_CIPHER_CTX_ctrl";

   -- Core Encrypt
   function EVP_EncryptInit_ex2 
     (ctx : System.Address; cipher : System.Address; key : System.Address; iv : System.Address; params : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_EncryptInit_ex2";

   function EVP_EncryptUpdate 
     (ctx : System.Address; out_buf : System.Address; out_len : System.Address; in_buf : System.Address; in_len : Interfaces.C.int) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_EncryptUpdate";

   function EVP_EncryptFinal_ex 
     (ctx : System.Address; out_buf : System.Address; out_len : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_EncryptFinal_ex";

   -- Core Decrypt
   function EVP_DecryptInit_ex2 
     (ctx : System.Address; cipher : System.Address; key : System.Address; iv : System.Address; params : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_DecryptInit_ex2";

   function EVP_DecryptUpdate 
     (ctx : System.Address; out_buf : System.Address; out_len : System.Address; in_buf : System.Address; in_len : Interfaces.C.int) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_DecryptUpdate";

   function EVP_DecryptFinal_ex 
     (ctx : System.Address; out_buf : System.Address; out_len : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_DecryptFinal_ex";

   --------------------------------------------------------------------
   -- MAC (HMAC) Management
   --------------------------------------------------------------------
   function EVP_MAC_fetch 
     (libctx : System.Address; name : Interfaces.C.Strings.chars_ptr; propq : System.Address) return System.Address
     with Import, Convention => C, External_Name => "EVP_MAC_fetch";

   function EVP_MAC_CTX_new (mac : System.Address) return System.Address
     with Import, Convention => C, External_Name => "EVP_MAC_CTX_new";

   function EVP_MAC_init 
     (ctx : System.Address; key : System.Address; key_len : Interfaces.C.size_t; params : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_MAC_init";

   function EVP_MAC_update 
     (ctx : System.Address; data : System.Address; data_len : Interfaces.C.size_t) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_MAC_update";

   function EVP_MAC_final 
     (ctx : System.Address; out_ptr : System.Address; out_len : System.Address; out_size : Interfaces.C.size_t) return Interfaces.C.int
     with Import, Convention => C, External_Name => "EVP_MAC_final";

   procedure EVP_MAC_CTX_free (ctx : System.Address)
     with Import, Convention => C, External_Name => "EVP_MAC_CTX_free";

   procedure EVP_MAC_free (mac : System.Address)
     with Import, Convention => C, External_Name => "EVP_MAC_free";

   --------------------------------------------------------------------
   -- Import liboqs (Post-Quantum)
   --------------------------------------------------------------------
   -- [KEM]
   function OQS_KEM_new (alg_name : Interfaces.C.Strings.chars_ptr) return System.Address
     with Import, Convention => C, External_Name => "OQS_KEM_new";

   function OQS_KEM_keypair (kem : System.Address; public_key : System.Address; secret_key : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "OQS_KEM_keypair";

   function OQS_KEM_encaps (kem : System.Address; ciphertext : System.Address; shared_secret : System.Address; public_key : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "OQS_KEM_encaps";

   function OQS_KEM_decaps (kem : System.Address; shared_secret : System.Address; ciphertext : System.Address; secret_key : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "OQS_KEM_decaps";

   procedure OQS_KEM_free (kem : System.Address)
     with Import, Convention => C, External_Name => "OQS_KEM_free";

   -- [SIG]
   function OQS_SIG_new (alg_name : Interfaces.C.Strings.chars_ptr) return System.Address
     with Import, Convention => C, External_Name => "OQS_SIG_new";

   function OQS_SIG_keypair (sig : System.Address; public_key : System.Address; secret_key : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "OQS_SIG_keypair";

   function OQS_SIG_sign (sig : System.Address; signature : System.Address; signature_len : System.Address;
                          message : System.Address; message_len : Interfaces.C.size_t; secret_key : System.Address) return Interfaces.C.int
     with Import, Convention => C, External_Name => "OQS_SIG_sign";

   function OQS_SIG_verify (sig : System.Address; message : System.Address; message_len : Interfaces.C.size_t;
                            signature : System.Address; signature_len : Interfaces.C.size_t; public_key : System.Address)
                            return Interfaces.C.int
     with Import, Convention => C, External_Name => "OQS_SIG_verify";

   procedure OQS_SIG_free (sig : System.Address)
     with Import, Convention => C, External_Name => "OQS_SIG_free";

   --------------------------------------------------------------------
   -- Khai báo các Subunits (separate)
   --------------------------------------------------------------------
   procedure AES_GCM_Encrypt
     (Key            : Key_32;
      Plaintext      : System.Address;
      Plaintext_Len  : Interfaces.C.size_t;
      AAD            : System.Address;
      AAD_Len        : Interfaces.C.size_t;
      Ciphertext_Out : out System.Address;
      Ciphertext_Len : out Interfaces.C.size_t;
      Status         : out Interfaces.C.int) is separate;

   procedure AES_GCM_Decrypt
     (Key            : Key_32;
      Ciphertext     : System.Address;
      Ciphertext_Len : Interfaces.C.size_t;
      AAD            : System.Address;
      AAD_Len        : Interfaces.C.size_t;
      Plaintext_Out  : out System.Address;
      Plaintext_Len  : out Interfaces.C.size_t;
      Status         : out Interfaces.C.int) is separate;

   procedure HMAC_SHA256
     (Key      : Key_32;
      Data     : System.Address;
      Data_Len : Interfaces.C.size_t;
      MAC_Out  : out Key_32;
      Status   : out Interfaces.C.int) is separate;

   procedure Kyber_Keypair
     (Public_Key : out Key_1568;
      Secret_Key : out Key_2400;
      Status     : out Interfaces.C.int) is separate;

   procedure Kyber_Encaps
     (Public_Key    : Key_1568;
      Ciphertext    : out Key_1312;
      Shared_Secret : out Key_32;
      Status        : out Interfaces.C.int) is separate;

   procedure Kyber_Decaps
     (Secret_Key    : Key_2400;
      Ciphertext    : Key_1312;
      Shared_Secret : out Key_32;
      Status        : out Interfaces.C.int) is separate;

   procedure Dilithium_Keypair
     (Public_Key : out Key_1952;
      Secret_Key : out Key_4032;
      Status     : out Interfaces.C.int) is separate;

   procedure Dilithium_Sign
     (Secret_Key   : Key_4032;
      Message      : System.Address;
      Message_Len  : Interfaces.C.size_t;
      Signature    : out Key_3309;
      Status       : out Interfaces.C.int) is separate;

   procedure Dilithium_Verify
     (Public_Key   : Key_1952;
      Message      : System.Address;
      Message_Len  : Interfaces.C.size_t;
      Signature    : Key_3309;
      Status       : out Interfaces.C.int) is separate;

   procedure Crypto_Free_Buffer (Ptr : System.Address) is separate;

end Crypto_Engine;