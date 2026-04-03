pragma Style_Checks (Off);
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with System;

separate (Crypto_Engine)

procedure Crypto_Free_Buffer (Ptr : System.Address) is

begin
   if Ptr /= System.Null_Address then
      C_Free (Ptr);
   end if;
end Crypto_Free_Buffer;