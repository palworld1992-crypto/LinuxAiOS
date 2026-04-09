--  SPARK_Mode (Off): Wrapper calls external liboqs library (OQS_KEM_keypair)
--  which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with Interfaces.C.Strings;
with System;

separate (Crypto_Engine)

procedure Kyber_Keypair
  (Public_Key : out Key_1568;
   Secret_Key : out Key_2400;
   Status     : out Interfaces.C.int) is

   use Interfaces.C.Strings;

   C_Name : chars_ptr := New_String (Kyber_Name_Str);
   kem    : System.Address;
   Result : Interfaces.C.int;
begin
   kem := OQS_KEM_new (C_Name);
   Free (C_Name);

   if kem = System.Null_Address then
      Status := -1;
      return;
   end if;

   Result := OQS_KEM_keypair (kem, Public_Key'Address, Secret_Key'Address);

   OQS_KEM_free (kem);
   Status := Result;
   return;

exception
   when others =>
      if kem /= System.Null_Address then
         OQS_KEM_free (kem);
      end if;
      Status := -1;
      return;
end Kyber_Keypair;