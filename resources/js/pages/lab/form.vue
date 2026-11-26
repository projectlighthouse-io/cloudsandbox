<script setup lang="ts">
import { useForm, usePage } from '@inertiajs/vue3'
import LabLayout from '../../layouts/LabLayout.vue'

const contact = useForm({ name: '', email: '', message: '' })
const newsletter = useForm({ email: '' })
const csrf = usePage<{ csrf_token: string | null }>().props.csrf_token

const send = () => contact.post('/lab/form/contact', { preserveScroll: true, onSuccess: () => contact.reset() })
const subscribe = () => newsletter.post('/lab/form/newsletter', { preserveScroll: true, errorBag: 'newsletter', onSuccess: () => newsletter.reset() })
</script>

<template>
  <LabLayout title="Forms & validation">
    <form class="card" @submit.prevent="send">
      <strong>Contact</strong> <span class="note">(Valid&lt;Json&lt;Contact&gt;&gt;)</span>
      <input v-model="contact.name" placeholder="Name" />
      <p v-if="contact.errors.name" class="error">{{ contact.errors.name }}</p>
      <input v-model="contact.email" placeholder="Email" />
      <p v-if="contact.errors.email" class="error">{{ contact.errors.email }}</p>
      <textarea v-model="contact.message" placeholder="Message (10 to 500 characters)" rows="3"></textarea>
      <p v-if="contact.errors.message" class="error">{{ contact.errors.message }}</p>
      <button type="submit" :disabled="contact.processing">Send</button>
    </form>

    <form class="card" @submit.prevent="subscribe">
      <strong>Newsletter</strong> <span class="note">(error bag "newsletter": its errors do not touch the contact form)</span>
      <input v-model="newsletter.email" placeholder="Email" />
      <p v-if="newsletter.errors.email" class="error">{{ newsletter.errors.email }}</p>
      <button type="submit" :disabled="newsletter.processing">Subscribe</button>
    </form>

    <form class="card" method="post" action="/lab/form/plain">
      <strong>Plain HTML form</strong> <span class="note">(no JavaScript; CSRF token in a hidden <code>_token</code> field)</span>
      <input type="hidden" name="_token" :value="csrf" />
      <input name="name" placeholder="Name" required />
      <button type="submit">Post</button>
    </form>
    <p class="note">
      Invalid input never reaches the handler: the server answers an Inertia visit with a 303 back here and the
      first error per field arrives in <code>props.errors</code>. Success flashes a message for one request.
    </p>
  </LabLayout>
</template>
