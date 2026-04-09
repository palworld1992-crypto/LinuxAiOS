--  SPARK_Mode (Off): Wrapper calls external liboqs library (OQS_SIG_sign/verify)
--  and uses dynamic memory allocation (malloc/free) which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C;
with Interfaces.C.Strings;
with System;

package body Identity_Manager is

   use type Interfaces.C.unsigned_long;

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

   type Supervisor_Keys is record
      Public_Key  : System.Address;
      Secret_Key  : System.Address;
      Expiry_Time : Interfaces.C.unsigned_long;
      Active      : Interfaces.C.int;
   end record;
   pragma Convention (C, Supervisor_Keys);

   type Supervisor_Keys_Array is array (1 .. 7) of Supervisor_Keys;
   Supervisors : Supervisor_Keys_Array;
   pragma Atomic (Supervisors);

   procedure Create_Token
     (Supervisor : Supervisor_ID;
      Expiry     : Interfaces.C.unsigned_long;
      Token_Out  : out Token_256;
      Status     : out Interfaces.C.int) is
      use Interfaces.C.Strings;
      C_Name : chars_ptr := New_String (Dilithium_Name_Str);
      sig : System.Address;
      Sig_Len_Ptr : System.Address;
      Result : Interfaces.C.int;
      Msg_Addr : System.Address;
      Current_Time : Interfaces.C.unsigned_long;
   begin
      if Supervisor = 0 or else Supervisor > 7 then
         Status := -1;
         Free (C_Name);
         return;
      end if;

      sig := OQS_SIG_new (C_Name);
      Free (C_Name);

      if sig = System.Null_Address then
         Status := -1;
         return;
      end if;

      Sig_Len_Ptr := C_Malloc (8);
      if Sig_Len_Ptr = System.Null_Address then
         OQS_SIG_free (sig);
         Status := -1;
         return;
      end if;

      Msg_Addr := C_Malloc (16);
      if Msg_Addr = System.Null_Address then
         C_Free (Sig_Len_Ptr);
         OQS_SIG_free (sig);
         Status := -1;
         return;
      end if;

      Current_Time := Expiry;
      declare
         Msg_Arr : array (1 .. 16) of Interfaces.C.unsigned_char;
         for Msg_Arr'Address use Msg_Addr;
      begin
         for I in 1 .. 8 loop
            Msg_Arr (I) := Interfaces.C.unsigned_char (Interfaces.C.unsigned_long'Pos (Supervisor) and 16#FF#);
            Msg_Arr (I + 8) := Interfaces.C.unsigned_char (Interfaces.C.unsigned_long'Pos (Current_Time) / (2**(8*(8-I))) and 16#FF#);
         end loop;
      end;

      Result := OQS_SIG_sign (sig, Token_Out'Address, Sig_Len_Ptr, Msg_Addr, 16, Supervisors (Integer (Supervisor)).Secret_Key);

      C_Free (Msg_Addr);
      C_Free (Sig_Len_Ptr);
      OQS_SIG_free (sig);

      if Result = 0 then
         Status := 0;
      else
         Status := -1;
      end if;
   exception
      when others =>
         Status := -1;
   end Create_Token;

   procedure Verify_Token
     (Token      : Token_256;
      Supervisor : Supervisor_ID;
      Status     : out Interfaces.C.int) is
      use Interfaces.C.Strings;
      C_Name : chars_ptr := New_String (Dilithium_Name_Str);
      sig : System.Address;
      Result : Interfaces.C.int;
      Msg_Addr : System.Address;
   begin
      if Supervisor = 0 or else Supervisor > 7 then
         Status := -2;
         Free (C_Name);
         return;
      end if;

      if Supervisors (Integer (Supervisor)).Public_Key = System.Null_Address then
         Status := -2;
         Free (C_Name);
         return;
      end if;

      sig := OQS_SIG_new (C_Name);
      Free (C_Name);

      if sig = System.Null_Address then
         Status := -2;
         return;
      end if;

      Msg_Addr := C_Malloc (16);
      if Msg_Addr = System.Null_Address then
         OQS_SIG_free (sig);
         Status := -2;
         return;
      end if;

      declare
         Msg_Arr : array (1 .. 16) of Interfaces.C.unsigned_char;
         for Msg_Arr'Address use Msg_Addr;
      begin
         for I in 1 .. 8 loop
            Msg_Arr (I) := Interfaces.C.unsigned_char (Interfaces.C.unsigned_long'Pos (Supervisor) and 16#FF#);
         end loop;
      end;

      Result := OQS_SIG_verify (sig, Msg_Addr, 16, Token, 256, Supervisors (Integer (Supervisor)).Public_Key);

      C_Free (Msg_Addr);
      OQS_SIG_free (sig);

      if Result = 0 then
         Status := 0;
      elsif Result = 1 then
         Status := -1;
      else
         Status := -2;
      end if;
   exception
      when others =>
         Status := -2;
   end Verify_Token;

end Identity_Manager;
