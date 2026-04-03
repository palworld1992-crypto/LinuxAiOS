with IDL_Types;
with Interfaces.C;

package body SCC_Type_Mapper is
   use type Interfaces.C.size_t;

   procedure Get_Type_Name
     (T : access constant IDL_Types.IDL_Type;
      Name : out Interfaces.C.int) is
   begin
      Name := Interfaces.C.int (IDL_Types.IDL_Kind'Pos (T.Kind));
   end Get_Type_Name;

   procedure Get_Alignment
     (T : access constant IDL_Types.IDL_Type;
      Align : out Interfaces.C.size_t) is
   begin
      case T.Kind is
         when IDL_Types.Kind_U8 | IDL_Types.Kind_I8 =>
            Align := 1;
         when IDL_Types.Kind_U16 | IDL_Types.Kind_I16 =>
            Align := 2;
         when IDL_Types.Kind_U32 | IDL_Types.Kind_I32 | IDL_Types.Kind_F32 =>
            Align := 4;
         when IDL_Types.Kind_U64 | IDL_Types.Kind_I64 | IDL_Types.Kind_F64 =>
            Align := 8;
         when IDL_Types.Kind_String =>
            Align := 8;
         when IDL_Types.Kind_Array =>
            if T.Element_Type /= null then
               Get_Alignment (T.Element_Type, Align);
            else
               Align := 1;
            end if;
         when IDL_Types.Kind_Struct =>
            Align := 8;
      end case;
   end Get_Alignment;

   procedure Map_Type
     (T : access constant IDL_Types.IDL_Type;
      Result : out Interfaces.C.size_t) is
      Total : Interfaces.C.size_t := 0;
   begin
      case T.Kind is
         when IDL_Types.Kind_U8 => Total := 1;
         when IDL_Types.Kind_U16 => Total := 2;
         when IDL_Types.Kind_U32 => Total := 4;
         when IDL_Types.Kind_U64 => Total := 8;
         when IDL_Types.Kind_I8 => Total := 1;
         when IDL_Types.Kind_I16 => Total := 2;
         when IDL_Types.Kind_I32 => Total := 4;
         when IDL_Types.Kind_I64 => Total := 8;
         when IDL_Types.Kind_F32 => Total := 4;
         when IDL_Types.Kind_F64 => Total := 8;
         when IDL_Types.Kind_String => Total := 8;
         when IDL_Types.Kind_Array =>
            if T.Element_Type /= null then
               declare
                  Element_Size : Interfaces.C.size_t;
               begin
                  Map_Type (T.Element_Type, Element_Size);
                  Total := Element_Size * T.Length;
               end;
            else
               Total := 0;
            end if;
         when IDL_Types.Kind_Struct =>
            if T.Fields /= null then
               for I in T.Fields.all'Range loop
                  declare
                     F : constant access IDL_Types.IDL_Field := T.Fields.all (I);
                     Field_Size : Interfaces.C.size_t;
                     End_Offset : Interfaces.C.size_t;
                  begin
                     if F.Type_Info /= null then
                        Map_Type (F.Type_Info, Field_Size);
                        End_Offset := F.Offset + Field_Size;
                        if End_Offset > Total then
                           Total := End_Offset;
                        end if;
                     end if;
                  end;
               end loop;
            end if;
      end case;
      Result := Total;
   end Map_Type;

end SCC_Type_Mapper;
