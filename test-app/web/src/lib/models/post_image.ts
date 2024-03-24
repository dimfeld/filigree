import { client, type ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const PostImageSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	file_size: z.number().int().optional(),
	file_hash: z.string().optional(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type PostImage = z.infer<typeof PostImageSchema>;
export const PostImagePopulatedGetSchema = PostImageSchema;
export type PostImagePopulatedGet = PostImage;
export const PostImagePopulatedListSchema = PostImageSchema;
export type PostImagePopulatedList = PostImage;
export const PostImageCreateResultSchema = PostImageSchema;
export type PostImageCreateResult = PostImage;

export const PostImageCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	file_size: z.number().int().optional(),
	file_hash: z.string().optional(),
	post_id: z.string().uuid(),
});

export type PostImageCreatePayloadAndUpdatePayload = z.infer<
	typeof PostImageCreatePayloadAndUpdatePayloadSchema
>;
export const PostImageCreatePayloadSchema =
	PostImageCreatePayloadAndUpdatePayloadSchema;
export type PostImageCreatePayload = PostImageCreatePayloadAndUpdatePayload;
export const PostImageUpdatePayloadSchema =
	PostImageCreatePayloadAndUpdatePayloadSchema;
export type PostImageUpdatePayload = PostImageCreatePayloadAndUpdatePayload;

export const baseUrl = "post_images";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const PostImageModel: ModelDefinition<typeof PostImageSchema> = {
	name: "PostImage",
	plural: "PostImages",
	baseUrl,
	urls,
	schema: PostImageSchema,
	createSchema: PostImageCreatePayloadSchema,
	updateSchema: PostImageUpdatePayloadSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "file_storage_key",
			type: "text",
			label: "File Storage Key",
			constraints: {
				required: true,
			},
		},
		{
			name: "file_storage_bucket",
			type: "text",
			label: "File Storage Bucket",
			constraints: {
				required: true,
			},
		},
		{
			name: "file_original_name",
			type: "text",
			label: "File Original Name",
			constraints: {
				required: false,
			},
		},
		{
			name: "file_size",
			type: "integer",
			label: "File Size",
			constraints: {
				required: false,
			},
		},
		{
			name: "file_hash",
			type: "blob",
			label: "File Hash",
			constraints: {
				required: false,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
};
