--  SPARK_Mode (Off): Wrapper calls external liboqs library (OQS_SIG_ml_dsa_65_verify)
--  which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with System;
with System.Address_To_Access_Conversions;

separate (Crypto_Engine)

procedure Dilithium_Verify
  (Public_Key   : System.Address;
   Message      : System.Address;
   Message_Len  : Interfaces.C.size_t;
   Signature    : System.Address;
   Signature_Len : Interfaces.C.size_t;
   Status_Ptr   : System.Address) is

   package Int_Conv is new System.Address_To_Access_Conversions(Interfaces.C.int);
   Status_Access : constant Int_Conv.Object_Pointer := Int_Conv.To_Pointer(Status_Ptr);
   Result : Interfaces.C.int;
begin
   -- Ensure liboqs is initialized
   Ensure_OQS_Initialized;
   
   if Signature = System.Null_Address or else Public_Key = System.Null_Address or else Status_Ptr = System.Null_Address then
      Status_Access.all := -1;
      return;
   end if;

   -- Use void procedure with out parameter for Ada compatibility
   OQS_ML_DSA_65_Verify_Out 
     (Message, 
      Message_Len, 
      Signature, 
      Signature_Len, 
      Public_Key,
      Result);

   Status_Access.all := Result;

exception
   when others =>
      Status_Access.all := -2;  -- Different error code for exception
end Dilithium_Verify;