pragma Style_Checks (Off);
with Interfaces.C;

package IDL_Types is
   pragma SPARK_Mode (On);

   type IDL_Kind is (Kind_U8, Kind_U16, Kind_U32, Kind_U64,
                     Kind_I8, Kind_I16, Kind_I32, Kind_I64,
                     Kind_F32, Kind_F64,
                     Kind_String, Kind_Array, Kind_Struct);
   pragma Convention (C, IDL_Kind);
   for IDL_Kind'Size use 32;

   type IDL_Type;
   type IDL_Field;

   -- Thêm các định nghĩa Access cụ thể để dùng cho Free
   type IDL_Type_Access is access all IDL_Type;
   type IDL_Field_Access is access all IDL_Field;
   type IDL_Field_Array is array (Interfaces.C.size_t range <>) of aliased IDL_Field_Access;
   type IDL_Field_Array_Access is access all IDL_Field_Array;

   type IDL_Field is record
      Name : Interfaces.C.char_array (1 .. 64);
      Type_Info : IDL_Type_Access; -- Cập nhật dùng IDL_Type_Access
      Offset : Interfaces.C.size_t;
   end record;

   type IDL_Type (Kind : IDL_Kind := Kind_U8) is record
      case Kind is
         when Kind_U8 | Kind_U16 | Kind_U32 | Kind_U64 |
              Kind_I8 | Kind_I16 | Kind_I32 | Kind_I64 |
              Kind_F32 | Kind_F64 | Kind_String =>
            null;
         when Kind_Array =>
            Element_Type : IDL_Type_Access; -- Cập nhật dùng IDL_Type_Access
            Length : Interfaces.C.size_t;
         when Kind_Struct =>
            Field_Count : Interfaces.C.size_t;
            Fields : IDL_Field_Array_Access; -- Cập nhật dùng IDL_Field_Array_Access
      end case;
   end record;

end IDL_Types;