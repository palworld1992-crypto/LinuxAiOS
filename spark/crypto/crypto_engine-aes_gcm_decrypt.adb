--  SPARK_Mode (Off): Wrapper calls external OpenSSL EVP API for AES-GCM decryption
--  which involves pointer manipulation and C library calls that cannot be verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C.Strings;
with System.Storage_Elements;

separate (Crypto_Engine)
procedure AES_GCM_Decrypt
  (Key             : Key_32;
   Ciphertext      : System.Address;
   Ciphertext_Len  : Interfaces.C.size_t;
   AAD             : System.Address;
   AAD_Len         : Interfaces.C.size_t;
   Plaintext_Buf   : System.Address;
   Plaintext_Buf_Size : Interfaces.C.size_t;
   Plaintext_Len   : out Interfaces.C.size_t;
   Status          : out Interfaces.C.int) is

   use Interfaces.C.Strings;
   use System.Storage_Elements;

   Cipher_Name_Str : constant String := "AES-256-GCM" & ASCII.NUL;
   Tag_Len_Const   : constant Interfaces.C.size_t := 16;
   IV_Len_Const    : constant Interfaces.C.int    := 12;

   EVP_CTRL_GCM_SET_IVLEN : constant Interfaces.C.int := 16#9#;
   EVP_CTRL_GCM_SET_TAG   : constant Interfaces.C.int := 16#11#;

   Cipher_C  : chars_ptr      := New_String (Cipher_Name_Str);
   Cipher    : System.Address := System.Null_Address;
   Ctx       : System.Address := System.Null_Address;
   
   IV        : aliased Key_12 := [others => Interfaces.C.char'Val(0)]; 
   
   Temp_Len  : aliased Interfaces.C.int := 0;
   Total_Len : Interfaces.C.int := 0;
   
   Actual_Cipher_Len : Interfaces.C.size_t;

   procedure Set_Error is
   begin
      Plaintext_Len := 0;
      Status := -1;
   end Set_Error;

begin
   -- 1. Validate input
   if Ciphertext_Len < Tag_Len_Const then
      if Cipher_C /= Null_Ptr then Free (Cipher_C); end if;
      Set_Error;
      return;
   end if;

   Actual_Cipher_Len := Ciphertext_Len - Tag_Len_Const;

   -- Check caller buffer
   if Plaintext_Buf = System.Null_Address
     or else Plaintext_Buf_Size < Actual_Cipher_Len
   then
      if Cipher_C /= Null_Ptr then Free (Cipher_C); end if;
      Set_Error;
      return;
   end if;

   -- 2. Fetch cipher
   Cipher := EVP_CIPHER_fetch (System.Null_Address, Cipher_C, System.Null_Address);
   Free (Cipher_C);
   
   if Cipher = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 3. Create context
   Ctx := EVP_CIPHER_CTX_new;
   if Ctx = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 4. Init
   if EVP_DecryptInit_ex2 (Ctx, Cipher, System.Null_Address, System.Null_Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 5. Set IV length
   if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_SET_IVLEN, IV_Len_Const, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 6. Set Key and IV
   if EVP_DecryptInit_ex2 (Ctx, System.Null_Address, Key'Address, IV'Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 7. AAD
   if AAD_Len > 0 then
      if EVP_DecryptUpdate (Ctx, System.Null_Address, Temp_Len'Address, AAD, Interfaces.C.int (AAD_Len)) = 0 then
         goto Error_Cleanup;
      end if;
   end if;

   -- 8. Decrypt into caller buffer
   if EVP_DecryptUpdate (Ctx, Plaintext_Buf, Temp_Len'Address, Ciphertext, Interfaces.C.int (Actual_Cipher_Len)) = 0 then
      goto Error_Cleanup;
   end if;
   Total_Len := Temp_Len;

   -- 9. Set expected tag
   declare
      Tag_Ptr : constant System.Address := Ciphertext + Storage_Offset (Actual_Cipher_Len);
   begin
      if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_SET_TAG, Interfaces.C.int (Tag_Len_Const), Tag_Ptr) = 0 then
         goto Error_Cleanup;
      end if;
   end;

   -- 10. Finalize (verifies tag)
   declare
      Final_Out_Ptr : constant System.Address := Plaintext_Buf + Storage_Offset (Total_Len);
      Res           : Interfaces.C.int;
   begin
      Res := EVP_DecryptFinal_ex (Ctx, Final_Out_Ptr, Temp_Len'Address);
      if Res <= 0 then
         goto Error_Cleanup;
      end if;
      Total_Len := Total_Len + Temp_Len;
   end;

   -- Success
   Plaintext_Len := Interfaces.C.size_t (Total_Len);
   Status        := 0;

   EVP_CIPHER_CTX_free (Ctx);
   return;

<<Error_Cleanup>>
   if Ctx /= System.Null_Address then 
      EVP_CIPHER_CTX_free (Ctx); 
   end if;
   Set_Error;
   return;

end AES_GCM_Decrypt;
