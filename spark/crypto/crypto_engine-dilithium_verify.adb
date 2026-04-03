pragma Style_Checks (Off);
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with Interfaces.C.Strings;
with System;

separate (Crypto_Engine)

procedure Dilithium_Verify
  (Public_Key   : Key_1952;
   Message      : System.Address;
   Message_Len  : Interfaces.C.size_t;
   Signature    : Key_3309;
   Status       : out Interfaces.C.int) is

   use Interfaces.C.Strings;

   C_Name : chars_ptr := New_String (Dilithium_Name_Str);
   sig    : System.Address;
   res    : Interfaces.C.int;
begin
   sig := OQS_SIG_new (C_Name);
   Free (C_Name);

   if sig = System.Null_Address then
      Status := -1;
      return;
   end if;

   res := OQS_SIG_verify 
     (sig, 
      Message, 
      Message_Len, 
      Signature'Address, 
      Signature'Length, 
      Public_Key'Address);

   OQS_SIG_free (sig);
   Status := res;
   return;

exception
   when others =>
      if sig /= System.Null_Address then
         OQS_SIG_free (sig);
      end if;
      Status := -1;
      return;
end Dilithium_Verify;