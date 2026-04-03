pragma Style_Checks (Off);
pragma SPARK_Mode (Off); -- Tắt SPARK cho phần body do sử dụng System.Address và nhãn goto 

with Interfaces.C.Strings;
with System.Storage_Elements;

separate (Crypto_Engine)
procedure AES_GCM_Decrypt
  (Key            : Key_32;
   Ciphertext     : System.Address;
   Ciphertext_Len : Interfaces.C.size_t;
   AAD            : System.Address;
   AAD_Len        : Interfaces.C.size_t;
   Plaintext_Out  : out System.Address;
   Plaintext_Len  : out Interfaces.C.size_t;
   Status         : out Interfaces.C.int) is

   use Interfaces.C.Strings;
   use System.Storage_Elements;

   -- Cấu hình tham số AES-GCM cho OpenSSL 3.x
   Cipher_Name_Str : constant String := "AES-256-GCM" & ASCII.NUL;
   Tag_Len_Const   : constant Interfaces.C.size_t := 16;
   IV_Len_Const    : constant Interfaces.C.int    := 12;

   EVP_CTRL_GCM_SET_IVLEN : constant Interfaces.C.int := 16#9#;
   EVP_CTRL_GCM_SET_TAG   : constant Interfaces.C.int := 16#11#;

   Cipher_C  : chars_ptr      := New_String (Cipher_Name_Str);
   Cipher    : System.Address := System.Null_Address;
   Ctx       : System.Address := System.Null_Address;
   Out_Buf   : System.Address := System.Null_Address;
   
   IV        : aliased Key_12 := [others => Interfaces.C.char'Val(0)]; 
   
   Temp_Len  : aliased Interfaces.C.int := 0;
   Total_Len : Interfaces.C.int := 0;
   
   Actual_Cipher_Len : Interfaces.C.size_t;

   procedure Set_Error is
   begin
      Plaintext_Out := System.Null_Address;
      Plaintext_Len := 0;
      Status := -1;
   end Set_Error;

begin
   -- 1. Kiểm tra độ dài đầu vào (Ciphertext phải chứa cả Tag)
   if Ciphertext_Len < Tag_Len_Const then
      if Cipher_C /= Null_Ptr then Free (Cipher_C); end if;
      Set_Error;
      return;
   end if;

   Actual_Cipher_Len := Ciphertext_Len - Tag_Len_Const;

   -- 2. Khởi tạo thuật toán (EVP_CIPHER)
   Cipher := EVP_CIPHER_fetch (System.Null_Address, Cipher_C, System.Null_Address);
   Free (Cipher_C);
   
   if Cipher = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 3. Tạo context mới
   Ctx := EVP_CIPHER_CTX_new;
   if Ctx = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 4. Khởi tạo quá trình giải mã
   if EVP_DecryptInit_ex2 (Ctx, Cipher, System.Null_Address, System.Null_Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 5. Thiết lập độ dài IV
   if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_SET_IVLEN, IV_Len_Const, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 6. Cung cấp Key và IV cho Context
   if EVP_DecryptInit_ex2 (Ctx, System.Null_Address, Key'Address, IV'Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 7. Xử lý AAD (Additional Authenticated Data)
   if AAD_Len > 0 then
      if EVP_DecryptUpdate (Ctx, System.Null_Address, Temp_Len'Address, AAD, Interfaces.C.int (AAD_Len)) = 0 then
         goto Error_Cleanup;
      end if;
   end if;

   -- 8. Cấp phát bộ nhớ cho Plaintext đầu ra
   Out_Buf := C_Malloc (Actual_Cipher_Len);
   if Out_Buf = System.Null_Address then
      goto Error_Cleanup;
   end if;

   -- 9. Giải mã Ciphertext (Không bao gồm Tag)
   if EVP_DecryptUpdate (Ctx, Out_Buf, Temp_Len'Address, Ciphertext, Interfaces.C.int (Actual_Cipher_Len)) = 0 then
      goto Error_Cleanup;
   end if;
   Total_Len := Temp_Len;

   -- 10. Thiết lập Tag dự kiến để OpenSSL kiểm tra tính toàn vẹn
   declare
      Tag_Ptr : constant System.Address := Ciphertext + Storage_Offset (Actual_Cipher_Len);
   begin
      if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_SET_TAG, Interfaces.C.int (Tag_Len_Const), Tag_Ptr) = 0 then
         goto Error_Cleanup;
      end if;
   end;

   -- 11. Hoàn tất giải mã (Kiểm tra xác thực Tag tại đây)
   declare
      Final_Out_Ptr : constant System.Address := Out_Buf + Storage_Offset (Total_Len);
      Res           : Interfaces.C.int;
   begin
      Res := EVP_DecryptFinal_ex (Ctx, Final_Out_Ptr, Temp_Len'Address);
      -- Nếu Res <= 0 nghĩa là Tag không khớp hoặc dữ liệu bị sửa đổi
      if Res <= 0 then
         goto Error_Cleanup;
      end if;
      Total_Len := Total_Len + Temp_Len;
   end;

   -- Gán kết quả thành công
   Plaintext_Out := Out_Buf;
   Plaintext_Len := Interfaces.C.size_t (Total_Len);
   Status        := 0;

   EVP_CIPHER_CTX_free (Ctx);
   return;

<<Error_Cleanup>>
   if Out_Buf /= System.Null_Address then 
      C_Free (Out_Buf); 
   end if;
   if Ctx /= System.Null_Address then 
      EVP_CIPHER_CTX_free (Ctx); 
   end if;
   Set_Error;
   return;

end AES_GCM_Decrypt;