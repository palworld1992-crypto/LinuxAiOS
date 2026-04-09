--  SPARK_Mode (Off): Wrapper calls external OpenSSL EVP API for AES-GCM encryption
--  which involves pointer manipulation and C library calls that cannot be verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C.Strings;
with System.Storage_Elements;

separate (Crypto_Engine)
procedure AES_GCM_Encrypt
  (Key             : Key_32;
   Plaintext       : System.Address;
   Plaintext_Len   : Interfaces.C.size_t;
   AAD             : System.Address;
   AAD_Len         : Interfaces.C.size_t;
   Ciphertext_Buf  : System.Address;
   Ciphertext_Buf_Size : Interfaces.C.size_t;
   Ciphertext_Len  : out Interfaces.C.size_t;
   Status          : out Interfaces.C.int) is

   use Interfaces.C.Strings;
   use System.Storage_Elements;

   Cipher_Name_Str : constant String := "AES-256-GCM" & ASCII.NUL;
   Tag_Len_Const   : constant Interfaces.C.size_t := 16;
   IV_Len_Const    : constant Interfaces.C.int    := 12;
   
   EVP_CTRL_GCM_SET_IVLEN : constant Interfaces.C.int := 16#9#;
   EVP_CTRL_GCM_GET_TAG   : constant Interfaces.C.int := 16#10#;
   
   Cipher_C  : chars_ptr      := New_String (Cipher_Name_Str);
   Cipher    : System.Address := System.Null_Address;
   Ctx       : System.Address := System.Null_Address;
   
   IV        : aliased Key_12 := [others => Interfaces.C.char'Val(0)]; 
   
   Temp_Len  : aliased Interfaces.C.int := 0;
   Total_Len : Interfaces.C.int := 0;

   procedure Set_Error is
   begin
      Ciphertext_Len := 0;
      Status := -1;
   end Set_Error;

begin
   -- Check buffer size
   if Ciphertext_Buf = System.Null_Address
     or else Ciphertext_Buf_Size < Plaintext_Len + Tag_Len_Const
   then
      Set_Error;
      return;
   end if;

   -- 1. Fetch cipher
   Cipher := EVP_CIPHER_fetch (System.Null_Address, Cipher_C, System.Null_Address);
   Free (Cipher_C);
   
   if Cipher = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 2. Create context
   Ctx := EVP_CIPHER_CTX_new;
   if Ctx = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 3. Init
   if EVP_EncryptInit_ex2 (Ctx, Cipher, System.Null_Address, System.Null_Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 4. Set IV length
   if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_SET_IVLEN, IV_Len_Const, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 5. Set Key and IV
   if EVP_EncryptInit_ex2 (Ctx, System.Null_Address, Key'Address, IV'Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 6. AAD
   if AAD_Len > 0 then
      if EVP_EncryptUpdate (Ctx, System.Null_Address, Temp_Len'Address, AAD, Interfaces.C.int (AAD_Len)) = 0 then
         goto Error_Cleanup;
      end if;
   end if;

   -- 7. Encrypt plaintext into caller buffer
   if EVP_EncryptUpdate (Ctx, Ciphertext_Buf, Temp_Len'Address, Plaintext, Interfaces.C.int (Plaintext_Len)) = 0 then
      goto Error_Cleanup;
   end if;
   Total_Len := Temp_Len;

   -- 8. Finalize
   declare
      Final_Out_Ptr : constant System.Address := Ciphertext_Buf + Storage_Offset (Total_Len);
   begin
      if EVP_EncryptFinal_ex (Ctx, Final_Out_Ptr, Temp_Len'Address) = 0 then
         goto Error_Cleanup;
      end if;
      Total_Len := Total_Len + Temp_Len;
   end;

   -- 9. Append tag
   declare
      Tag_Ptr : constant System.Address := Ciphertext_Buf + Storage_Offset (Total_Len);
   begin
      if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_GET_TAG, Interfaces.C.int (Tag_Len_Const), Tag_Ptr) = 0 then
         goto Error_Cleanup;
      end if;
   end;

   -- Success
   Ciphertext_Len := Interfaces.C.size_t (Total_Len) + Tag_Len_Const;
   Status         := 0;

   if Ctx /= System.Null_Address then
      EVP_CIPHER_CTX_free (Ctx);
   end if;
   return;

<<Error_Cleanup>>
   if Ctx /= System.Null_Address then 
      EVP_CIPHER_CTX_free (Ctx); 
   end if;
   Set_Error;
   return;

end AES_GCM_Encrypt;
