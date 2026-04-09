--  SPARK_Mode (Off): Wrapper calls external OpenSSL EVP_MAC API for HMAC-SHA256
--  which involves pointer manipulation and C library calls that cannot be verified by SPARK.
pragma SPARK_Mode (Off);

with Interfaces.C; use Interfaces.C;
with Interfaces.C.Strings;
with System;
with System.Storage_Elements; use System.Storage_Elements; 

separate (Crypto_Engine)

procedure HMAC_SHA256
  (Key      : Key_32;
   Data     : System.Address;
   Data_Len : Interfaces.C.size_t;
   MAC_Out  : out Key_32;
   Status   : out Interfaces.C.int) is

   use Interfaces.C.Strings;

   mac_name    : constant String := "HMAC" & ASCII.NUL;
   digest_name : constant String := "SHA256" & ASCII.NUL;
   param_key   : constant String := "digest" & ASCII.NUL;
   
   mac_c       : chars_ptr := New_String (mac_name);
   digest_c    : chars_ptr := New_String (digest_name);
   p_key_c     : chars_ptr := New_String (param_key);
   
   mac         : System.Address := System.Null_Address;
   ctx         : System.Address := System.Null_Address;
   res         : Interfaces.C.int := 0;
   
   Params      : array (0 .. 1) of OSSL_PARAM;
   OSSL_PARAM_UTF8_STRING : constant Interfaces.C.unsigned := 4;

begin
   Params(0) := (Key         => p_key_c, 
                 Data_Type   => OSSL_PARAM_UTF8_STRING, 
                 Data        => To_Addr (digest_c),
                 Data_Size   => Interfaces.C.size_t (digest_name'Length - 1), 
                 Return_Size => 0);
                 
   Params(1) := (Key         => Null_Ptr,
                 Data_Type   => 0,
                 Data        => System.Null_Address,
                 Data_Size   => 0,
                 Return_Size => 0);

   mac := EVP_MAC_fetch (System.Null_Address, mac_c, System.Null_Address);
   Free (mac_c); 
   
   if mac = System.Null_Address then
      goto Error_Cleanup;
   end if;

   ctx := EVP_MAC_CTX_new (mac);
   if ctx = System.Null_Address then
      goto Error_Cleanup;
   end if;

   res := EVP_MAC_init (ctx, Key'Address, Key'Length, Params(0)'Address);
   
   Free (digest_c); digest_c := Null_Ptr;
   Free (p_key_c);  p_key_c  := Null_Ptr;
   
   if res = 0 then
      goto Error_Cleanup;
   end if;

   res := EVP_MAC_update (ctx, Data, Data_Len);
   if res = 0 then
      goto Error_Cleanup;
   end if;

   declare
      out_len : aliased Interfaces.C.size_t := 0;
   begin
      res := EVP_MAC_final (ctx, MAC_Out'Address, out_len'Address, MAC_Out'Length);
   end;

   if res = 0 then
      goto Error_Cleanup;
   end if;

   EVP_MAC_CTX_free (ctx);
   EVP_MAC_free (mac);
   Status := 0;
   return;

<<Error_Cleanup>>
   if digest_c /= Null_Ptr then Free (digest_c); end if;
   if p_key_c /= Null_Ptr then Free (p_key_c); end if;
   if ctx /= System.Null_Address then EVP_MAC_CTX_free (ctx); end if;
   if mac /= System.Null_Address then EVP_MAC_free (mac); end if;
   Status := -1;
   return;

exception
   when others =>
      if digest_c /= Null_Ptr then Free (digest_c); end if;
      if p_key_c /= Null_Ptr then Free (p_key_c); end if;
      if ctx /= System.Null_Address then EVP_MAC_CTX_free (ctx); end if;
      if mac /= System.Null_Address then EVP_MAC_free (mac); end if;
      Status := -1;
      return;
end HMAC_SHA256;