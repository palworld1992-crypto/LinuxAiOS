pragma Style_Checks (Off);
pragma SPARK_Mode (Off); -- Tắt SPARK cho phần body do sử dụng System.Address và nhãn goto

with Interfaces.C.Strings;
with System.Storage_Elements;

separate (Crypto_Engine)
procedure AES_GCM_Encrypt
  (Key            : Key_32;
   Plaintext      : System.Address;
   Plaintext_Len  : Interfaces.C.size_t;
   AAD            : System.Address;
   AAD_Len        : Interfaces.C.size_t;
   Ciphertext_Out : out System.Address;
   Ciphertext_Len : out Interfaces.C.size_t;
   Status         : out Interfaces.C.int) is

   use Interfaces.C.Strings;
   use System.Storage_Elements;

   -- Cấu hình tham số AES-GCM cho OpenSSL 3.x
   Cipher_Name_Str : constant String := "AES-256-GCM" & ASCII.NUL;
   Tag_Len_Const   : constant Interfaces.C.size_t := 16;
   IV_Len_Const    : constant Interfaces.C.int    := 12;
   
   EVP_CTRL_GCM_SET_IVLEN : constant Interfaces.C.int := 16#9#;
   EVP_CTRL_GCM_GET_TAG   : constant Interfaces.C.int := 16#10#;

   Cipher_C  : chars_ptr      := New_String (Cipher_Name_Str);
   Cipher    : System.Address := System.Null_Address;
   Ctx       : System.Address := System.Null_Address;
   Out_Buf   : System.Address := System.Null_Address;
   
   IV        : aliased Key_12 := [others => Interfaces.C.char'Val(0)]; 
   
   Temp_Len  : aliased Interfaces.C.int := 0;
   Total_Len : Interfaces.C.int := 0;

   -- Helper để gán out parameters trong các nhánh lỗi
   procedure Set_Error is
   begin
      Ciphertext_Out := System.Null_Address;
      Ciphertext_Len := 0;
      Status := -1;
   end Set_Error;

begin
   -- 1. Khởi tạo thuật toán (EVP_CIPHER)
   Cipher := EVP_CIPHER_fetch (System.Null_Address, Cipher_C, System.Null_Address);
   Free (Cipher_C); -- Giải phóng chuỗi tên thuật toán ngay sau khi dùng
   
   if Cipher = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 2. Tạo context mới cho quá trình mã hóa
   Ctx := EVP_CIPHER_CTX_new;
   if Ctx = System.Null_Address then
      Set_Error;
      return;
   end if;

   -- 3. Khởi tạo quá trình mã hóa
   if EVP_EncryptInit_ex2 (Ctx, Cipher, System.Null_Address, System.Null_Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 4. Thiết lập độ dài IV (mặc định là 12 bytes cho GCM)
   if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_SET_IVLEN, IV_Len_Const, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 5. Cung cấp Key và IV cho Context
   if EVP_EncryptInit_ex2 (Ctx, System.Null_Address, Key'Address, IV'Address, System.Null_Address) = 0 then
      goto Error_Cleanup;
   end if;

   -- 6. Xử lý dữ liệu bổ sung (AAD - Additional Authenticated Data)
   if AAD_Len > 0 then
      if EVP_EncryptUpdate (Ctx, System.Null_Address, Temp_Len'Address, AAD, Interfaces.C.int (AAD_Len)) = 0 then
         goto Error_Cleanup;
      end if;
   end if;

   -- 7. Cấp phát bộ nhớ cho đầu ra: Ciphertext + Tag
   Out_Buf := C_Malloc (Plaintext_Len + Tag_Len_Const);
   if Out_Buf = System.Null_Address then
      goto Error_Cleanup;
   end if;

   -- 8. Mã hóa Plaintext
   if EVP_EncryptUpdate (Ctx, Out_Buf, Temp_Len'Address, Plaintext, Interfaces.C.int (Plaintext_Len)) = 0 then
      goto Error_Cleanup;
   end if;
   Total_Len := Temp_Len;

   -- 9. Hoàn tất mã hóa (Finalize)
   declare
      Final_Out_Ptr : constant System.Address := Out_Buf + Storage_Offset (Total_Len);
   begin
      if EVP_EncryptFinal_ex (Ctx, Final_Out_Ptr, Temp_Len'Address) = 0 then
         goto Error_Cleanup;
      end if;
      Total_Len := Total_Len + Temp_Len;
   end;

   -- 10. Lấy Authentication Tag và ghi vào cuối buffer
   declare
      Tag_Ptr : constant System.Address := Out_Buf + Storage_Offset (Total_Len);
   begin
      if EVP_CIPHER_CTX_ctrl (Ctx, EVP_CTRL_GCM_GET_TAG, Interfaces.C.int (Tag_Len_Const), Tag_Ptr) = 0 then
         goto Error_Cleanup;
      end if;
   end;

   -- Gán kết quả đầu ra
   Ciphertext_Out := Out_Buf;
   Ciphertext_Len := Interfaces.C.size_t (Total_Len) + Tag_Len_Const;
   Status         := 0;

   -- Giải phóng context
   if Ctx /= System.Null_Address then
      EVP_CIPHER_CTX_free (Ctx);
   end if;
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

end AES_GCM_Encrypt;