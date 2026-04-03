pragma Style_Checks (Off);
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with Interfaces.C.Strings;
with System;

separate (Crypto_Engine)

procedure Dilithium_Sign
  (Secret_Key   : Key_4032;
   Message      : System.Address;
   Message_Len  : Interfaces.C.size_t;
   Signature    : out Key_3309;
   Status       : out Interfaces.C.int) is

   use Interfaces.C.Strings;

   C_Name         : chars_ptr := New_String (Dilithium_Name_Str);
   Sig_Obj        : System.Address;
   Actual_Sig_Len : aliased Interfaces.C.size_t := Signature'Length;
   Result         : Interfaces.C.int; 
begin
   Sig_Obj := OQS_SIG_new (C_Name);
   Free (C_Name);

   if Sig_Obj = System.Null_Address then
      Status := -1;
      return;
   end if;

   Result := OQS_SIG_sign 
     (Sig_Obj, 
      Signature'Address, 
      Actual_Sig_Len'Address, 
      Message, 
      Message_Len, 
      Secret_Key'Address);

   OQS_SIG_free (Sig_Obj);
   Status := Result;
   return;

exception
   when others =>
      if Sig_Obj /= System.Null_Address then
         OQS_SIG_free (Sig_Obj);
      end if;
      Status := -1;
      return;
end Dilithium_Sign;