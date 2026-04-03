with IDL_Types;
with SCC_Type_Mapper;
with Interfaces.C;

package body SCC_Layout_Calculator is
   use type IDL_Types.IDL_Kind;
   use type Interfaces.C.size_t;
   use type Interfaces.C.int;

   procedure Compute_Total_Size
     (Struct : access IDL_Types.IDL_Type;
      Total_Size : out Interfaces.C.size_t) is
      Total : Interfaces.C.size_t := 0;
   begin
      if Struct.Kind /= IDL_Types.Kind_Struct then
         Total_Size := 0;
         return;
      end if;

      if Struct.Fields /= null then
         for I in Struct.Fields.all'Range loop
            declare
               F : constant access IDL_Types.IDL_Field := Struct.Fields.all (I);
               Field_Size : Interfaces.C.size_t;
            begin
               if F.Type_Info /= null then
                  SCC_Type_Mapper.Map_Type (F.Type_Info, Field_Size);
                  Total := F.Offset + Field_Size;
               end if;
            end;
         end loop;
      end if;

      Total_Size := Total;
   end Compute_Total_Size;

   procedure Compute_Field_Offset
     (Struct : access IDL_Types.IDL_Type;
      Field_Index : Interfaces.C.size_t;
      Offset : out Interfaces.C.size_t;
      Status : out Interfaces.C.int) is
   begin
      Offset := 0;
      Status := -1;

      if Struct.Kind /= IDL_Types.Kind_Struct then
         return;
      end if;

      if Struct.Fields = null or else Field_Index = 0 then
         return;
      end if;

      if Field_Index > Struct.Field_Count then
         return;
      end if;

      declare
         F : constant access IDL_Types.IDL_Field := Struct.Fields.all (Integer (Field_Index));
      begin
         Offset := F.Offset;
         Status := 0;
      end;
   end Compute_Field_Offset;

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
               SCC_Type_Mapper.Get_Alignment (F.Type_Info, Field_Align);
               if Field_Align > Max_Align then
                  Max_Align := Field_Align;
               end if;

               Offset := (Offset + Field_Align - 1) / Field_Align * Field_Align;
               F.Offset := Offset;

               declare
                  Size : Interfaces.C.size_t;
               begin
                  SCC_Type_Mapper.Map_Type (F.Type_Info, Size);
                  Offset := Offset + Size;
               end;
            end if;
         end;
      end loop;

      Offset := (Offset + Max_Align - 1) / Max_Align * Max_Align;
   end Compute_Layout;

end SCC_Layout_Calculator;
