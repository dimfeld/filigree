import { z } from 'zod';

export const LoginFormSchema = z.object({
  email: z.string().email(),
  password: z.string().optional(),
});

export interface LoginFormResponse {
  email: string;
  password?: string;
}
