pragma Style_Checks (Off);
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with Interfaces.C.Strings;
with System;

separate (Crypto_Engine)

procedure Dilithium_Keypair 
  (Public_Key : out Key_1952;
   Secret_Key : out Key_4032;
   Status     : out Interfaces.C.int) is

   use Interfaces.C.Strings;

   C_Name : chars_ptr := New_String (Dilithium_Name_Str);
   sig    : System.Address;
   Result : Interfaces.C.int;
begin
   sig := OQS_SIG_new (C_Name);
   Free (C_Name);

   if sig = System.Null_Address then
      Status := -1;
      return;
   end if;

   Result := OQS_SIG_keypair (sig, Public_Key'Address, Secret_Key'Address);

   OQS_SIG_free (sig);
   Status := Result;
   return;

exception
   when others =>
      if sig /= System.Null_Address then
         OQS_SIG_free (sig);
      end if;
      Status := -1;
      return;
end Dilithium_Keypair;