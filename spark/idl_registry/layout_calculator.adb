--  SPARK_Mode (Off): Calls Type_Mapper.Map_Type which has SPARK_Mode (Off)
--  due to access type operations. Cannot be formally verified by SPARK.
pragma SPARK_Mode (Off);

with IDL_Types;
with Type_Mapper;
with Interfaces.C;

package body Layout_Calculator is
   use type IDL_Types.IDL_Kind;
   use type Interfaces.C.size_t;

   procedure Compute_Layout (Struct : access IDL_Types.IDL_Type) is
      Offset : Interfaces.C.size_t := 0;
      Max_Align : Interfaces.C.size_t := 1;
      Field_Align : Interfaces.C.size_t;
   begin
      if Struct.Kind /= IDL_Types.Kind_Struct then
         return;
      end if;

      if Struct.Fields = null then
         return;
      end if;

      for I in Struct.Fields.all'Range loop
         declare
            F : constant access IDL_Types.IDL_Field := Struct.Fields.all (I);
         begin
            if F.Type_Info /= null then
               case F.Type_Info.Kind is
                  when IDL_Types.Kind_U8 | IDL_Types.Kind_I8 =>
                     Field_Align := 1;
                  when IDL_Types.Kind_U16 | IDL_Types.Kind_I16 =>
                     Field_Align := 2;
                  when IDL_Types.Kind_U32 | IDL_Types.Kind_I32 | IDL_Types.Kind_F32 =>
                     Field_Align := 4;
                  when others =>
                     Field_Align := 8;
               end case;

               if Field_Align > Max_Align then
                  Max_Align := Field_Align;
               end if;

               Offset := (Offset + Field_Align - 1) / Field_Align * Field_Align;
               F.Offset := Offset;

               declare
                  Size : Interfaces.C.size_t;
               begin
                  Type_Mapper.Map_Type (F.Type_Info, Size);
                  Offset := Offset + Size;
               end;
            end if;
         end;
      end loop;

      Offset := (Offset + Max_Align - 1) / Max_Align * Max_Align;
   end Compute_Layout;

end Layout_Calculator;
