--  SPARK_Mode (Off): Uses access types and pointer operations
--  which cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with IDL_Types;
with Interfaces.C;
use IDL_Types;
use type Interfaces.C.size_t;

package body Type_Mapper is

   procedure Map_Type
     (T : access constant IDL_Type;
      Result : out Interfaces.C.size_t) is
      Total : Interfaces.C.size_t := 0;
   begin
      if T = null then
         Result := 0;
         return;
      end if;

      case T.Kind is
         when Kind_U8 =>
            Total := 1;
         when Kind_U16 =>
            Total := 2;
         when Kind_U32 =>
            Total := 4;
         when Kind_U64 =>
            Total := 8;
         when Kind_I8 =>
            Total := 1;
         when Kind_I16 =>
            Total := 2;
         when Kind_I32 =>
            Total := 4;
         when Kind_I64 =>
            Total := 8;
         when Kind_F32 =>
            Total := 4;
         when Kind_F64 =>
            Total := 8;
         when Kind_String =>
            Total := 8;
         when Kind_Array =>
            if T.Element_Type /= null then
               declare
                  Elem_Size : Interfaces.C.size_t;
               begin
                  Map_Type (T.Element_Type, Elem_Size);
                  Total := Elem_Size * T.Length;
               end;
            end if;
         when Kind_Struct =>
            null;
      end case;
      Result := Total;
   end Map_Type;

end Type_Mapper;
