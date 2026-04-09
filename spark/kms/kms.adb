--  SPARK_Mode (Off): Wrapper calls external liboqs library (OQS_SIG_keypair/sign/verify)
--  and uses dynamic memory allocation (malloc/free) which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with System;
with Interfaces.C;
with Interfaces.C.Strings;

package body KMS is

   use type Interfaces.C.size_t;
   use type Interfaces.C.int;

   procedure C_Free (Ptr : System.Address)
     with Import, Convention => C, External_Name => "free";

   function C_Malloc (Size : Interfaces.C.size_t) return System.Address
     with Import, Convention => C, External_Name => "malloc";

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

   Dilithium_Name_Str : constant String := "ML-DSA-65" & ASCII.NUL;

   procedure Generate_Key
     (Key_Out     : System.Address;
      Key_Out_Len : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
   is
      use Interfaces.C.Strings;
      C_Name : chars_ptr := New_String (Dilithium_Name_Str);
      sig : System.Address;
      Result : Interfaces.C.int;
   begin
      if Key_Out_Len < KEY_SIZE then
         Status := STATUS_ERROR;
         Free (C_Name);
         return;
      end if;

      sig := OQS_SIG_new (C_Name);
      Free (C_Name);

      if sig = System.Null_Address then
         Status := STATUS_ERROR;
         return;
      end if;

      Result := OQS_SIG_keypair (sig, Key_Out, Key_Out);

      OQS_SIG_free (sig);

      if Result = 0 then
         Status := STATUS_SUCCESS;
      else
         Status := STATUS_ERROR;
      end if;
   exception
      when others =>
         if sig /= System.Null_Address then
            OQS_SIG_free (sig);
         end if;
         Status := STATUS_ERROR;
   end Generate_Key;

   procedure Sign_Data
     (Data_In     : System.Address;
      Data_In_Len : Interfaces.C.size_t;
      Sig_Out     : System.Address;
      Sig_Out_Len : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
   is
      use Interfaces.C.Strings;
      C_Name : chars_ptr := New_String (Dilithium_Name_Str);
      sig : System.Address;
      Sig_Len_Ptr : System.Address;
      Result : Interfaces.C.int;
   begin
      if Data_In = System.Null_Address or else
         Sig_Out = System.Null_Address or else
         Data_In_Len > MAX_DATA_LEN or else
         Sig_Out_Len < SIG_SIZE
      then
         Status := STATUS_ERROR;
         return;
      end if;

      sig := OQS_SIG_new (C_Name);
      Free (C_Name);

      if sig = System.Null_Address then
         Status := STATUS_ERROR;
         return;
      end if;

      Sig_Len_Ptr := C_Malloc (8);
      if Sig_Len_Ptr = System.Null_Address then
         OQS_SIG_free (sig);
         Status := STATUS_ERROR;
         return;
      end if;

      Result := OQS_SIG_sign (sig, Sig_Out, Sig_Len_Ptr, Data_In, Data_In_Len, Sig_Out);

      C_Free (Sig_Len_Ptr);
      OQS_SIG_free (sig);

      if Result = 0 then
         Status := STATUS_SUCCESS;
      else
         Status := STATUS_ERROR;
      end if;
   exception
      when others =>
         if sig /= System.Null_Address then
            OQS_SIG_free (sig);
         end if;
         Status := STATUS_ERROR;
   end Sign_Data;

   procedure Verify_Signature
     (Data_In     : System.Address;
      Data_In_Len : Interfaces.C.size_t;
      Sig_In      : System.Address;
      Sig_In_Len  : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
   is
      use Interfaces.C.Strings;
      C_Name : chars_ptr := New_String (Dilithium_Name_Str);
      sig : System.Address;
      Result : Interfaces.C.int;
   begin
      if Data_In = System.Null_Address or else
         Sig_In = System.Null_Address or else
         Data_In_Len > MAX_DATA_LEN or else
         Sig_In_Len < SIG_SIZE
      then
         Status := STATUS_ERROR;
         return;
      end if;

      sig := OQS_SIG_new (C_Name);
      Free (C_Name);

      if sig = System.Null_Address then
         Status := STATUS_ERROR;
         return;
      end if;

      Result := OQS_SIG_verify (sig, Data_In, Data_In_Len, Sig_In, Sig_In_Len, Data_In);

      OQS_SIG_free (sig);

      if Result = 0 then
         Status := STATUS_SUCCESS;
      else
         Status := STATUS_ERROR;
      end if;
   exception
      when others =>
         if sig /= System.Null_Address then
            OQS_SIG_free (sig);
         end if;
         Status := STATUS_ERROR;
   end Verify_Signature;

end KMS;
